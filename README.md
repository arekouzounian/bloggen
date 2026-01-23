![logo](logo.svg)

BlogGen 
--- 
A simplified blogging framework for personal use. 

---
**Table of Contents**
- [Project Structure](#project-structure)
- [Installation](#installation)
  - [Server-side](#server-side)
  - [Client-Side](#client-side)
- [Usage](#usage)
- [Testing](#testing)
---

The rough ideas for the project can be found in [the idea doc](./idea-doc.md), where I initially put my ideas for the project. 

This framework is intended to allow one to set-up and add to a simple web-based blog quickly. The idea is to write blog posts in markdown, and have a set of tools and software communicating with each other to allow quick, painless blog post uploading to a central server. 

The goal was for these tools to be developed for my own personal blog, but a secondary goal is to allow this set of tools to be configurable in such a way that anyone could use this to deploy their own personal blog. 

The web-based frontend will be housed in the [frontend folder](./bloggen-frontend/), the command-line interface code will be in [the cli folder](./cli/), and  server-side code exists in [the server folder](./server/) 



# Project Structure
This project is essentially three main parts: 
1. A [CLI client](./cli/), written in Go, that helps to generate post structures and update/upload them
2. An [SSH server](./server/), written in Rust, that implements a modified version of SFTP that handles uploads from the client and places them according to a JSON configuration
3. A [Next.js frontend](./bloggen-frontend/), written in React, that implements a web frontend which displays posts and handles page data/server-side page generation.



# Installation 
To install, you must first have SSH access to the target machine that will be hosting the blog. As of right now, the only SSH auth method is SSH key auth, so you need to have a keypair that can authenticate to the server properly. 

Additionally, you need to have docker installed on the server.

Once you have that, you can do the following:

## Server-side 
1. **Clone the repo**:
```sh
# Via HTTPS
$ git clone https://github.com/arekouzounian/bloggen.git
```
2. **Compose the containers**:
```sh
# Go into the freshly cloned directory
$ cd bloggen

# Compose w/ docker 
$ docker compose build 
```
3. **Run**:
```sh
$ docker compose up --detach
```

**To Shutdown, if needed**:
```sh
docker compose down
```

## Client-Side 
1. **Clone the repo**:
```sh
# Via HTTPS
$ git clone https://github.com/arekouzounian/bloggen.git

```
2. **Configure CLI**:
```sh
# Enter freshly cloned repo
$ cd ./bloggen/cli

# Build; requires Go >= 1.20 to be installed
$ go build 

# At this point, you can use the tool locally...
$ ./bloggen <flags> <commmand> 
$ ./bloggen --help
# ...or you can copy the executable into a folder included in your PATH
$ cp ./bloggen /usr/local/bin
$ bloggen <flags> <command> 
$ bloggen --help
```
3. **Cleanup**: \
At this point, you can do remove the `bloggen-server` and `cli` folders on your client machine (or the entire directory if you've already copied the executable into your PATH). \
On your server, you can remove the `cli` directory if you'd like, as you won't need it. However, it's important to note that blog posts will be populated and stored in the `bloggen-frontend/app/static` folder. 

# Usage 
The basic usage loop is as follows: 
1. On the **client**, use `bloggen post init <postname>` to create an empty post. See `bloggen post init -h` for more information on its usage. 
2. Once you've created a post directory, you can edit the file `/path/to/post/<postname>.md`, where `<postname>` is whatever you titled the post with the previous command. If you don't specify an output directory, the post structure will be created as a new folder in the current directory. 
3. Once you're done editing the markdown file, you can use `bloggen post upload -t path/to/post_directory` to upload it to the server. 
   - **Important note:** there are 3 things needed to upload the post, each of which are specified via flags: 
    1. A **keyfile** for SSH auth, which defaults to ~/.ssh/id_rsa,
    2. A **target directory**, which must have the canonical structure provided by `bloggen post init`, and
    3. A **server target**, which uses `ip:port` format, and defaults to `localhost:2222`. This target can be specified with a flag, but can also draw from the `BLOGGEN_SERVER` environment variable. Once you have your server code running, you could do: 
    ```sh 
        # if using bash
        echo export BLOGGEN_SERVER=<your_server_ip>:2222 >> ~/.bash_profile

        # for zsh
        echo export BLOGGEN_SERVER=<your_server_ip>:2222 >> ~/.zshrc
    ```
    The port defaults to port 2222, but if you'd like to change it, you would need to change it in the above declaration, as well as the `port` key in the  `server/docker.json` file. If you do make this change, make sure to re-build the docker containers accordingly.

    If you're already in the post directory, you needn't specify the target flag, and can simply run `bloggen post upload`, assuming the other information has been configured/defaulted correctly. 

# Testing

BlogGen has a comprehensive test suite that includes unit tests and integration tests.

## Quick Start

Run all tests with a single command:
```bash
./run-tests.sh
```

Or use Make:
```bash
make test
```

## Test Categories

### Unit Tests
Test individual components in isolation:

```bash
# Run all unit tests (client + server in parallel)
./run-tests.sh --unit
# or
make test-unit

# Run only client tests
make test-client

# Run only server tests
make test-server
```

### Integration Tests
Test complete workflows end-to-end:

```bash
# Run all integration tests
./run-tests.sh --integration
# or
make test-integration

# Run specific integration tests
make test-e2e-client    # Client-only tests (no server required)
make test-e2e-full      # Full client + server tests
make test-e2e-fuse      # FUSE filesystem tests
```

## Test Options

The unified test runner supports several options:

```bash
# Run with verbose output
./run-tests.sh --verbose

# Run unit tests sequentially instead of parallel
./run-tests.sh --sequential

# Run specific test suites
./run-tests.sh --e2e-client
./run-tests.sh --e2e-full
./run-tests.sh --e2e-fuse
```

## Environment Variables

Integration tests support configuration via environment variables:

```bash
# Skip database setup (use existing DB)
SKIP_DB_SETUP=1 ./run-tests.sh --integration

# Keep database running after tests
KEEP_DB_RUNNING=1 ./run-tests.sh --integration

# Use custom server port
SERVER_PORT=8080 ./run-tests.sh --integration
```

## Test Structure

- **Unit Tests**: Located in `src/` directories as Rust test modules
  - Client: `client/bgc/src/**/*.rs` (106+ tests)
  - Server: `v2-server/src/**/*.rs` (33+ tests)
  
- **Integration Tests**: Located in `tst/` directory
  - `e2e-client.sh` - Client-only workflow tests
  - `e2e-full.sh` - Complete client + server integration
  - `e2e-fuse.sh` - FUSE filesystem integration

For more details on integration tests, see [tst/README.md](./tst/README.md).

## Continuous Integration

All tests are designed to run in CI environments. See [tst/README.md](./tst/README.md) for CI configuration examples. 
