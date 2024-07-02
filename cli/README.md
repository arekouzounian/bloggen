# bloggen CLI
A simple command line interface written in Go, using gomarkdown and cobra.

This folder will contain all the files necessary to use the CLI tool. 

To run the tool: 
```bash
$ go run main.go <cmd> <flags> <args>
```
To see the help text:
```bash 
$ go run main.go < -h | --help >
```

To build the tool into an executable: 
```bash
$ go build
```
This will create a `bloggen` executable in the same directory. You may then choose to add this executable to your PATH to make it usable from any directory: 

```sh
# if using bash 
$ echo "export PATH=$PATH:/path/to/bloggen" >> ~/.bash_profile

# if using zsh 
$ echo "export PATH=$PATH:/path/to/bloggen" >> ~/.zshrc

# you can alternatively symlink into a directory that is already in your PATH
# this example uses /usr/bin/ but you can use another one
$ ln -s /path/to/bloggen /usr/bin/bloggen
```


You can then use the tool as follows: 
```bash
$ ./bloggen <cmd> <flags> <args> # if in the same directory as executable

$ bloggen <cmd> <flags> <args> # if in global PATH 
```
*Please note that the '$' is not necessary in any of the above commands, and is used to indicate the beginning of the shell prompt.*

### Features: 
- markdown file to html conversion (`conv` command)
- post initialization with canonical format (`post init` command)
- post uploading to the bloggen server, with automatic conversion (`post upload` command)



