# runr
[![build](https://github.com/paasim/runr/workflows/build/badge.svg)](https://github.com/paasim/runr/actions)

Tool for running scripts defined in yaml files. This can be used as a [post-receive hook](https://git-scm.com/docs/githooks#post-receive) on git remotes to achieve CI-type of behavior.

## Installation

Recommended way is to install the `deb`-package from [release builds](https://github.com/paasim/runr/releases). This includes [`podman`](https://podman.io/), which is used to run the tasks inside a container. Other options could be supported, but are not for now.

Building with `cargo` also works, but then `podman` has to be installed manually.

## Usage

For `git push` to trigger the runs, three things are needed:
1. `runr` installed on the git remote (and available on path)
2. `runr.yaml`-file on the repository root
3. A post-receive hook on the remote

### `runr.yaml`-file

`runr.yaml` should be placed on the repository root and look as follows:

```yaml
# optionally specify maximum number of parallel tasks, defaults to 1
# 0 means the number of cores on the machine
n_parallel: 2
# optionally specify default image for the task. if none is specified,
# the commands are run directly on the matchine, ie. not inside a container.
# the images must have /bin/bash to run the commands
image: "docker.io/library/debian:bookworm-slim"
tasks:
- commands: |
    echo starting step 1a
    sleep 1
    echo ending step 1a
  name: "step-1a" # name of the task
  image: "docker.io/library/debian:bookworm" # optionally override image per task
- commands: |
    echo starting step 2
    sleep 3
    echo ending step 2
  name: "step-2"
- commands: |
    echo starting step 1b
    sleep 1
    echo ending step 1b
  name: "step-1b"
  depends: ["step-1a"] # specify dependencies for the step
```

### post-receive hook

The hook should be executable, placed at `hooks/post-receive` on the remote, and look roughly as follows:

```sh
#!/bin/sh
while read -r OLD_OID NEW_OID BRANCH; do
  BRANCH=${BRANCH#refs/heads/} runr
done
```

### Example

The `Makefile` contains steps for testing the behavior locally. Note that this assumes that `runr` is already installed and available on the path.

```sh
$ make test-hook # add a local bare repo and push to it, triggering the hook

Enumerating objects: 51, done.
Counting objects: 100% (51/51), done.
Delta compression using up to 8 threads
Compressing objects: 100% (48/48), done.
Writing objects: 100% (51/51), 24.16 KiB | 6.04 MiB/s, done.
Total 51 (delta 6), reused 0 (delta 0), pack-reused 0 (from 0)
remote: Trying to pull docker.io/library/debian:bookworm...
remote: Getting image source signatures
remote: Copying blob sha256:cf05a52c02353f0b2b6f9be0549ac916c3fb1dc8d4bacd405eac7f28562ec9f2
remote: Copying config sha256:b2ab84c007feae81d95c5350d44ad7a54ea4693a79cb40fb05bd3fe00cbd4d26
remote: Writing manifest to image destination
remote: b2ab84c007feae81d95c5350d44ad7a54ea4693a79cb40fb05bd3fe00cbd4d26
remote: step-2  | starting step 2
remote: step-1a | starting step 1a
remote: step-1a | ending step 1a
remote: step-1b | starting step 1b
remote: step-1b | ending step 1b
remote: step-2  | ending step 2
To runr/tmp/runr-bare
 * [new branch]      main -> main

$ make clean-bare # remove the temporary remote afterwards
```
