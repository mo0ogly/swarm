#include <errno.h>
#include <linux/bpf.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void) {
 union bpf_attr attr = {0};
 attr.map_type=BPF_MAP_TYPE_ARRAY;attr.key_size=4;attr.value_size=8;attr.max_entries=1;
 long fd=syscall(__NR_bpf,BPF_MAP_CREATE,&attr,sizeof(attr));int failure=fd<0?errno:0;
 if(fd>=0)close((int)fd);
 printf("{\"probe\":\"BPF_MAP_CREATE\",\"available\":%s,\"errno\":%d,\"collection_started\":false,\"foreign_processes_observed\":false}\n",fd>=0?"true":"false",failure);
 return 0;
}
