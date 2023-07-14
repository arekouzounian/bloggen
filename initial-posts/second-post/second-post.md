# Back to Working 
---
For anyone reading this, no time has passed at all (since the blog is still not up), but for me, several weeks have passed since I was working on this project. I lost a good bit of motivation, and have been enjoying my summer with a bunch of different activities and friends. 

Regardless, it's time I got back to working on this project, and in my brief time back working, I've pivoted from working on the frontend to working on the backend. My last few commits have been to essentially rip one of the examples from the yew framework into some sort of working frontend mock-up, but I got pretty burnt out and frustrated. I was essentially copy-pasting, and not really doing any learning or understanding. Additionally, it became apparent that having a mock-up of the frontend wouldn't really work well if I had no content to have it dynamically display, so I decided to go back to figuring out server-side stuff. 

I've been doing my best recently to learn how daemons work, and while some initial cursory google searches (and overcomplicated networking class from last semester) had me worried about the complexity of implementing a daemon in rust, further research showed that systemd will solve all my problems (...surely?). 

From here on out I'll be working on implementing a basic socket listening application in Rust, with the future goal of having it listen for connections and accept incoming data (blog posts). I also want it to be easily configurable with some sort of central config file that it reads from, but I'll start with a basic default config. 

One major concern going forward is security--if I just make the daemon listen through a tcp socket with the eventual goal of downloading files onto the server, there are clear possibilities for RCE and other major vulnerabilities if I'm not careful. As of right now, my strategy to avoid this is to use some sort of SSH authentication. Doing a quick look at a popular ssh package in Rust, it doesn't seem like it'll be too hard to do. 



## Notes and Links for myself 
- [Useful so - systemd](https://stackoverflow.com/questions/61443052/rust-daemon-best-practices)
- [Systemd spec/reference](https://www.freedesktop.org/software/systemd/man/daemon.html)
- [Rust ssh package](https://docs.rs/ssh/latest/ssh/)