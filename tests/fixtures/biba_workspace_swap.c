#define _GNU_SOURCE
#include <dlfcn.h>
#include <fcntl.h>
#include <stdarg.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include <sys/stat.h>
/* Owned-fixture fault injection only: swap the inspected leaf before open64. */
int open64(const char *path, int flags, ...) {
 static int (*original)(const char*,int,...); static int injected;
 if(!original) original=dlsym(RTLD_NEXT,"open64");
 mode_t mode=0; if(flags&O_CREAT){va_list args;va_start(args,flags);mode=va_arg(args,int);va_end(args);}
 const char *workspace=getenv("BIBA_FIXTURE_WORKSPACE");
 if(!injected && workspace && strcmp(path,workspace)==0 && (flags&O_PATH)){
  injected=1;char parked[4096],alias[4096];
  snprintf(parked,sizeof parked,"%s.retained",workspace);snprintf(alias,sizeof alias,"%s/alias",workspace);
  if(rename(workspace,parked)||mkdir(workspace,0700)||link(getenv("BIBA_FIXTURE_SYNTHETIC"),alias)) _exit(91);
 }
 return original(path,flags,mode);
}
