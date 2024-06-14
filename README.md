# bloggen 
--- 
A simplified blogging framework for personal use. 

The rough ideas for the project can be found in [the idea doc](./idea-doc.md), where I initially put my ideas for the project. 

This framework is intended to allow one to set-up and add to a simple web-based blog quickly. The idea is to write blog posts in markdown, and have a set of tools and software communicating with each other to allow quick, painless blog post uploading to a central server. 

The goal was for these tools to be developed for my own personal blog, but a secondary goal is to allow this set of tools to be configurable in such a way that anyone could use this to deploy their own personal blog. 

The web-based frontend will be housed in the [frontend folder](./bloggen-frontend/), the command-line interface code will be in [the cli folder](./cli/), and  server-side code exists in [the server folder](./server/) 

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

## Client-Side: 
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
