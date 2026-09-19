//go:build linux

package main

import (
	"context"
	"database/sql"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"syscall"
	"time"
)

type ManagedRepositoryRequest struct {
	Path          string   `json:"path"`
	CommittedOnly bool     `json:"committed_only"`
	Include       []string `json:"include_dirty,omitempty"`
}
type ManagedRepository struct {
	Subdir        string   `json:"project_subdir,omitempty"`
	Source        string   `json:"source"`
	Storage       string   `json:"storage"`
	Base          string   `json:"base_commit"`
	Candidate     string   `json:"candidate_commit"`
	Snapshot      string   `json:"snapshot_digest"`
	Included      []string `json:"included,omitempty"`
	RequestDigest string   `json:"request_digest"`
}
type ManagedAttempt struct {
	Agent  string `json:"agent_id"`
	Work   string `json:"work_id"`
	Task   string `json:"task_id"`
	Base   string `json:"base_commit"`
	Path   string `json:"path"`
	State  string `json:"state"`
	Result string `json:"result_commit"`
	Detail string `json:"detail"`
}

func managedGit(dir string, args ...string) (string, error) {
	ctx, cancel := context.WithTimeout(context.Background(), 90*time.Second)
	defer cancel()
	cmd := exec.CommandContext(ctx, "git", append([]string{"-c", "core.hooksPath=/dev/null", "-c", "commit.gpgsign=false", "-c", "protocol.file.allow=always"}, args...)...)
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}
	cmd.Cancel = func() error {
		if cmd.Process == nil {
			return nil
		}
		return syscall.Kill(-cmd.Process.Pid, syscall.SIGKILL)
	}
	cmd.WaitDelay = time.Second
	cmd.Dir = dir
	cmd.Env = append(os.Environ(), "GIT_TERMINAL_PROMPT=0", "GIT_AUTHOR_NAME=Swarm", "GIT_AUTHOR_EMAIL=swarm@localhost", "GIT_COMMITTER_NAME=Swarm", "GIT_COMMITTER_EMAIL=swarm@localhost")
	out, err := cmd.CombinedOutput()
	if err != nil {
		return "", fmt.Errorf("git %s : %s", args[0], guardBlock(string(out), 2000))
	}
	return strings.TrimSpace(string(out)), nil
}
func managedLock(root, work string) (func(), error) {
	if !safeName(work) {
		return nil, fmt.Errorf("identifiant de mission invalide")
	}
	dir := filepath.Join(root, ".swarm", "managed", work)
	if err := os.MkdirAll(dir, 0700); err != nil {
		return nil, err
	}
	f, err := os.OpenFile(filepath.Join(dir, "operation.lock"), os.O_CREATE|os.O_RDWR, 0600)
	if err != nil {
		return nil, err
	}
	if err = syscall.Flock(int(f.Fd()), syscall.LOCK_EX|syscall.LOCK_NB); err != nil {
		f.Close()
		return nil, fmt.Errorf("une opération Git est déjà en cours pour cette mission")
	}
	return func() { _ = syscall.Flock(int(f.Fd()), syscall.LOCK_UN); f.Close() }, nil
}
func (s *Store) configureManagedRepository(work string, r ManagedRepositoryRequest) (*ManagedRepository, error) {
	unlock, err := managedLock(s.root, work)
	if err != nil {
		return nil, err
	}
	defer unlock()
	source, err := resolveWorkspace(s.root, r.Path)
	if err != nil {
		return nil, err
	}
	top, err := managedGit(source, "rev-parse", "--show-toplevel")
	if err != nil {
		return nil, err
	}
	subdir, err := filepath.Rel(top, source)
	if err != nil || strings.HasPrefix(subdir, "..") {
		return nil, fmt.Errorf("projet hors dépôt Git")
	}
	if subdir == "." {
		subdir = ""
	}
	encoded, _ := json.Marshal(r)
	digest := hash(encoded)
	storage := filepath.Join(s.root, ".swarm", "managed", work)
	manifest := filepath.Join(storage, "repository.json")
	if raw, err := os.ReadFile(manifest); err == nil {
		var prior ManagedRepository
		if err = json.Unmarshal(raw, &prior); err != nil {
			return nil, err
		}
		if prior.RequestDigest != digest {
			return nil, fmt.Errorf("la base de mission est déjà figée avec une autre sélection")
		}
		if _, e := managedGit(filepath.Join(prior.Storage, "repository.git"), "cat-file", "-e", prior.Base+"^{commit}"); e != nil {
			return nil, e
		}
		return &prior, nil
	}
	if raw, e := os.ReadFile(manifest + ".pending"); e == nil {
		var prior ManagedRepository
		if json.Unmarshal(raw, &prior) == nil && prior.RequestDigest == digest {
			if _, e = managedGit(filepath.Join(storage, "repository.git"), "cat-file", "-e", prior.Base+"^{commit}"); e == nil {
				if e = atomicWrite(manifest, raw); e != nil {
					return nil, e
				}
				return &prior, nil
			}
		}
	}
	base, err := managedGit(source, "rev-parse", "HEAD")
	if err != nil {
		return nil, err
	}
	dirty, err := managedGit(source, "status", "--porcelain", "--untracked-files=normal")
	if err != nil {
		return nil, err
	}
	if dirty != "" && !r.CommittedOnly && len(r.Include) == 0 {
		return nil, fmt.Errorf("dépôt modifié : choisir le commit enregistré ou nommer les fichiers de l’instantané")
	}
	if r.CommittedOnly && len(r.Include) > 0 {
		return nil, fmt.Errorf("choisir le commit seul ou une liste de fichiers, pas les deux")
	}
	staging, err := os.MkdirTemp(storage, "repository-build-")
	if err != nil {
		return nil, err
	}
	defer os.RemoveAll(staging)
	bare := filepath.Join(staging, "repository.git")
	if _, err = managedGit(storage, "clone", "--bare", "--no-local", "--no-hardlinks", top, bare); err != nil {
		return nil, err
	}
	included := map[string]string{}
	if len(r.Include) > 0 {
		copyDir := filepath.Join(staging, "snapshot")
		if _, err = managedGit(storage, "clone", "--no-local", bare, copyDir); err != nil {
			return nil, err
		}
		if _, err = managedGit(copyDir, "checkout", "--detach", base); err != nil {
			return nil, err
		}
		total := int64(0)
		for _, name := range r.Include {
			if name == "" || filepath.IsAbs(name) || filepath.Clean(name) != name || name == ".." || strings.HasPrefix(name, "../") || strings.HasPrefix(name, ".git/") || strings.HasPrefix(name, ".swarm/") || name == ".git" || name == ".swarm" {
				return nil, fmt.Errorf("chemin d’instantané interdit : %s", name)
			}
			if _, err = managedGit(source, "check-ignore", "--", name); err == nil {
				return nil, fmt.Errorf("fichier ignoré exclu de l’instantané : %s", name)
			}
			src := filepath.Join(source, name)
			dest := filepath.Join(copyDir, subdir, name)
			for parent := dest; parent != copyDir; parent = filepath.Dir(parent) {
				if st, e := os.Lstat(parent); e == nil && st.Mode()&os.ModeSymlink != 0 {
					return nil, fmt.Errorf("lien interdit dans la destination : %s", name)
				}
			}
			stat, statErr := os.Lstat(src)
			if os.IsNotExist(statErr) {
				if _, err = managedGit(source, "ls-files", "--error-unmatch", "--", name); err != nil {
					return nil, err
				}
				if err = os.Remove(dest); err != nil && !os.IsNotExist(err) {
					return nil, err
				}
				included[name] = "deleted"
				continue
			}
			if statErr != nil || !stat.Mode().IsRegular() {
				return nil, fmt.Errorf("fichier régulier requis : %s", name)
			}
			if _, err = localFile(source, name); err != nil {
				return nil, err
			}
			total += stat.Size()
			if total > 64<<20 {
				return nil, fmt.Errorf("instantané supérieur à 64 Mio")
			}
			data, err := os.ReadFile(src)
			if err != nil {
				return nil, err
			}
			if err = os.MkdirAll(filepath.Dir(dest), 0700); err != nil {
				return nil, err
			}
			if err = os.WriteFile(dest, data, stat.Mode().Perm()); err != nil {
				return nil, err
			}
			included[name] = hash(data)
		}
		gitPaths := []string{}
		for _, name := range r.Include {
			gitPaths = append(gitPaths, filepath.Join(subdir, name))
		}
		if _, err = managedGit(copyDir, append([]string{"add", "-A", "--"}, gitPaths...)...); err != nil {
			return nil, err
		}
		tree, err := managedGit(copyDir, "write-tree")
		if err != nil {
			return nil, err
		}
		base, err = managedGit(copyDir, "commit-tree", tree, "-p", base, "-m", "Instantané autorisé pour Swarm")
		if err != nil {
			return nil, err
		}
		if _, err = managedGit(bare, "fetch", "--no-tags", copyDir, base); err != nil {
			return nil, err
		}
	}
	if _, err = managedGit(bare, "update-ref", "refs/heads/swarm-result", base); err != nil {
		return nil, err
	}
	if _, err = managedGit(bare, "symbolic-ref", "HEAD", "refs/heads/swarm-result"); err != nil {
		return nil, err
	}
	final := filepath.Join(storage, "repository.git")
	if _, err = os.Lstat(final); err == nil {
		return nil, fmt.Errorf("dépôt préparé sans manifeste : examiner avant reprise")
	}
	snap, _ := json.Marshal(map[string]any{"base": base, "files": included})
	repo := &ManagedRepository{Subdir: subdir, Source: source, Storage: storage, Base: base, Candidate: base, Snapshot: hash(snap), Included: r.Include, RequestDigest: digest}
	raw, _ := json.Marshal(repo)
	if err = atomicWrite(manifest+".pending", raw); err != nil {
		return nil, err
	}
	if err = os.Rename(bare, final); err != nil {
		return nil, err
	}
	if err = atomicWrite(manifest, raw); err != nil {
		return nil, err
	}
	return repo, nil
}
func managedCopyPath(repo *ManagedRepository, task string, attempt int) string {
	return filepath.Join(managedCopyRoot(repo, task, attempt), repo.Subdir)
}
func managedCopyRoot(repo *ManagedRepository, task string, attempt int) string {
	return filepath.Join(repo.Storage, "copies", fmt.Sprintf("%s-%d", task, attempt))
}
func (s *Store) managedAttempt(id string) (ManagedAttempt, error) {
	var a ManagedAttempt
	err := s.db.QueryRow("SELECT agent_id,work_id,task_id,base_commit,path,state,result_commit,detail FROM managed_attempts WHERE agent_id=?", id).Scan(&a.Agent, &a.Work, &a.Task, &a.Base, &a.Path, &a.State, &a.Result, &a.Detail)
	return a, err
}
func (s *Store) ensureManagedAttempt(w Work, r Launch) (string, error) {
	repo, err := s.managedRepository(w)
	if err != nil {
		return "", err
	}
	unlock, err := managedLock(s.root, w.ID)
	if err != nil {
		return "", err
	}
	defer unlock()
	if prior, e := s.managedAttempt(r.EventID); e == nil {
		if prior.Work != w.ID || prior.Task != r.TaskID {
			return "", fmt.Errorf("copie attribuée à une autre tâche")
		}
		if prior.State != "ready" {
			return "", fmt.Errorf("copie non disponible : %s", prior.State)
		}
		if filepath.Dir(prior.Path) != filepath.Join(repo.Storage, "copies") {
			return "", fmt.Errorf("copie hors dépôt géré")
		}
		actual, e := filepath.EvalSymlinks(prior.Path)
		if e != nil || actual != prior.Path {
			return "", fmt.Errorf("copie absente ou redirigée")
		}
		if st, e := os.Lstat(filepath.Join(prior.Path, ".git")); e != nil || !st.IsDir() {
			return "", fmt.Errorf("métadonnées Git non isolées")
		}
		return filepath.Join(prior.Path, repo.Subdir), nil
	} else if e != sql.ErrNoRows {
		return "", e
	}
	task, err := w.task(r.TaskID)
	if err != nil {
		return "", err
	}
	path := managedCopyRoot(repo, task.ID, len(task.Attempts)+1)
	manifest := filepath.Join(repo.Storage, "copy-"+r.EventID+".json")
	if _, err = os.Lstat(path); err == nil {
		raw, e := os.ReadFile(manifest)
		if e != nil {
			return "", fmt.Errorf("copie existante sans attribution ; ne pas écraser")
		}
		var prior ManagedAttempt
		if e = json.Unmarshal(raw, &prior); e != nil || prior.Agent != r.EventID || prior.Work != w.ID || prior.Task != task.ID || prior.Path != path {
			return "", fmt.Errorf("attribution de copie incohérente")
		}
		_, e = s.db.Exec("INSERT INTO managed_attempts(agent_id,work_id,task_id,base_commit,path,state) VALUES(?,?,?,?,?,'ready')", prior.Agent, prior.Work, prior.Task, prior.Base, prior.Path)
		if e != nil {
			return "", e
		}
		return filepath.Join(path, repo.Subdir), nil
	}
	if err = os.MkdirAll(filepath.Dir(path), 0700); err != nil {
		return "", err
	}
	temp, err := os.MkdirTemp(filepath.Dir(path), "prepare-")
	if err != nil {
		return "", err
	}
	defer os.RemoveAll(temp)
	clone := filepath.Join(temp, "copy")
	if _, err = managedGit(repo.Storage, "clone", "--no-local", "--no-hardlinks", filepath.Join(repo.Storage, "repository.git"), clone); err != nil {
		return "", err
	}
	if _, err = managedGit(clone, "checkout", "--detach", repo.Candidate); err != nil {
		return "", err
	}
	record, _ := json.Marshal(ManagedAttempt{Agent: r.EventID, Work: w.ID, Task: task.ID, Base: repo.Candidate, Path: path, State: "ready"})
	if err = atomicWrite(manifest, record); err != nil {
		return "", err
	}
	if err = os.Rename(clone, path); err != nil {
		return "", err
	}
	_, err = s.db.Exec("INSERT INTO managed_attempts(agent_id,work_id,task_id,base_commit,path,state) VALUES(?,?,?,?,?,'ready')", r.EventID, w.ID, task.ID, repo.Candidate, path)
	if err != nil {
		return "", err
	}
	return filepath.Join(path, repo.Subdir), nil
}
func (s *Store) managedFailure(a Agent, reason string) error {
	_, err := s.db.Exec("UPDATE managed_attempts SET state='conflict',detail=? WHERE agent_id=? AND state!='integrated'", reason, a.ID)
	if err != nil {
		return err
	}
	w, err := s.get(a.WorkID)
	if err != nil {
		return err
	}
	raw, _ := json.Marshal(map[string]string{"agent": a.ID, "reason": reason})
	_, err = s.mutate(w.ID, "managed.conflict", "managed-fail-"+hash(raw)[:32], w.Revision, raw, func(w *Work) error {
		task, e := w.task(a.TaskID)
		if e != nil {
			return e
		}
		if !currentTaskAttempt(task, a.Attempt) {
			return fmt.Errorf("tentative remplacée")
		}
		task.Status = "blocked"
		task.Blocker = reason
		task.Next = "Le responsable doit proposer une correction sur la révision actuelle."
		scope, _ := w.Planning.scope(task.ScopeID)
		scope.State = "ready"
		w.Planning.Inbox = append(w.Planning.Inbox, PlanningEvent{ID: planningEventID(a.ID, reason), Scope: scope.ID, Kind: "integration_failed", Task: task.ID, Attempt: a.Attempt, Message: guardBlock(reason, 4000), At: now()})
		return nil
	})
	return err
}
