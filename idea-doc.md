# bloggen 
A blogging framework.

### Idea
- write blogs in markdown locally 
- use CLI tool to send new blog posts to server, converted into HTML
- server (dockerized) listens for connections and directs the files accordingly 
- blog is served using apache or similar webserver software 
- Due to containerization, can be duplicated easily 

### Components
- Webserver/Frontend
- Blog upload listener 
- CLI Tool 

#### Proposed Stack 
- Apache Webserver 
- [Yew](https://github.com/yewstack/yew) for frontend 
- Rust upload listener 
- Go CLI Tool (Cobra)

#### Extended Ideas
- CLI Tool additions:
  - delete post
  - download all posts
  - modify post 
- Other frontends
  - Since the primary way of uploading blog posts is predicated upon a CLI tool and markdown, the tool could be extended to have a web frontend
  - WYSIWYG, web-based editor to create blog posts 
  - would likely require authentication and introduce security vulnerabilities, so this is an idea yet to be fully explored 


### Coming back to this project in 2024
New plan: 
- uses go command line client to upload to listening server
  - listening server is custom, but maybe auths by checking clients ssh key 
  - For SSH auth and encryption: [rust lib](https://github.com/RustCrypto/SSH)
  - [This lib seems to do a lot of what I want](https://docs.rs/thrussh/latest/thrussh/)
  - [This lib is a fork of the last one that is actually on github and not on some weird proprietary fucked up website](https://github.com/warp-tech/russh)
- server builds static content from new code, deploys it to given directory which is served from apache or similar 
- Proposed stack: 
  - Go client 
  - Rust server 
    - can be dockerized 
    - essentially an ssh server wrapper 
  - apache httpd 
    - also can run from docker container 
  - Frontend: 
    - react.js (or yew if I'm really feeling masochistic)
  - Stretch: 
    - if httpd & server code running on containers, maybe package up within kubernetes? 
