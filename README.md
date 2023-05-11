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

