# bloggen 
A blogging framework.

### Coming back to this project in 2024
I'm leaving the below unchanged to give insight as to what the project idea initially was. Though the idea hasn't changed, the actual stack did get modified quite a bit. Currently the stack is as follows: 
- Go CLI client (using Cobra)
- Rust SSH/SFTP server (using russh)
- Next.js frontend (with the possibility of using Express down the line; but I'm trying to make all this with as little dependencies as possible)
- (Not Implemented Yet) Docker for containerization of frontend and server 

At the time of writing this update, I'm mainly focused on making the frontend (and learning next.js/tailwind, for that matter). The hardest part has been trying to get proper semi-static server-side-generation of page data, as it would be much better for SEO to use static pages whenever possible. 

The ultimate goal for the usage of this project from the user perspective is as follows: 
1. The user downloads the server and frontend, ideally housed in docker containers, and runs them on a remote server that they are capable of SSH'ing into. Both containers can be configured quickly and minimally using a couple of config files (likely JSON)
2. Assuming DNS configuration and domain registration is already in place for whatever domain the user wants the website to be reachable at, all they have to do is simply using the `bloggen post init` and `bloggen post upload` commands to create posts and upload them from their local machines. 
3. Immediately, the new blog posts are accessible from the public website as static pages. 

When I first came up with the idea of this project about a year ago, I didn't know much about web technologies, having barely made my own website in pure HTML/CSS/JS. Thus, I had no idea that static site generators like Jekyll and Hugo existed, and worked in a very similar way. However, I went ahead with making this project for a few reasons: 
1. It's an excuse to learn Rust, Next.js, Tailwind, and to practice Go
2. It works without being tied to a git repository and CI
3. It is modular and can be extended to have more functionality (due to the server being built on top of SFTP), so later on down the line I can add things like a GUI editor to make uploading posts more streamlined 
4. It requires very little to get off the ground; just SSH access to a server and a domain name. 

I wouldn't include this on the list as a legitimate reason, but it's also probably true that making an overcomplicated personal blog is some sort of rite of passage for most programmers. 

## Initial Version 
---
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
---

<!-- New plan: 
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
 -->

