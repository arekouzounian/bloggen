# Returning from a "hiatus"
---
Before today, the last commit to my bloggen project was about a year ago. So, what happened? 

Well, school was a big part of it, but a sizeable part was also due to my inability to come up with a solid plan on how to design this project. Before, I had been making small projects and executables without any semblance of how larger scale projects worked. I was good at brainstorming ideas for the project, but ultimately, my ability to design the software and integrate it into a tech stack was lacking. 

My research process was *(and still is)* somewhat flawed; I could come up with what seemed like good ideas at first, but when I went to implement them, they began to become quickly contrived. I didn't scope out the right libraries first, and was approaching the whole project with a very hacky mindset. Because of that, I kept repeating mistakes and falling into design traps. 

I went from trying to have no authentication and just use pure TCP, to wanting to build over SSH, to wanting to write my own low level server code (and defining my own HTTP-adjacent protocol for it), to just forgetting the server code entirely and trying to make it all work with a clever client and a couple of scripts, and back and forth multiple times. Finally, I've settled on this design: 

1. The server will essentially be an SFTP server using the `russh` library. The only assumption to be made is that the client will have their public key in the `authorized_keys` section on the server; this is a reasonable assumption to make for any cloud scenario.
    - I wasn't planning on doing password auth initially, but it's definitely something I could add in the future. 
2. The client will remain simple, just generating metadata and post structure as it's already been implemented, then securely transmitting the result to the server. 
3. The appeal of using the server itself will be due to a configuration system, like simply modifying some JSON to change properties of the blog. 
4. The frontend is undecided as of right now but honestly isn't too high on the priority list; it can be swapped out for whatever. The only major difference here is that I'll likely be opting for a static website generation method; the server will statically build the website and put the new build artifacts in the right place. 
5. The eventual goal is to have the server and frontend work together within separate docker containers. Whether that will mean learning how to coordinate using Kubernetes, or just having some clever scripts that set things up in the right locations with the right docker volumes, I'm not sure. 
6. Ultimately, I want the server/frontend code to be able to be 'plugged in' to any cloud server, and all the user will have to do is modify some JSON and make sure their website DNS is set up correctly through whatever provider they use. Then, they can write posts on their local computer(s) and upload them painlessly, securely, having everything else handled, and their post will show up on the website instantly. 

Let's see how this all goes. 