# Hello, World! 
---
This is my very first blog post, and it's being written in markdown. I thought I'd take this opportunity to explain my idea for this project, while the project is being built! In fact, at this point in time, the only thing I've implemented so far is the ability to convert markdown files to HTML using the CLI tool. Let's take a look at what I'm trying to build! 

## Website History 
It started with my first website; an idea to make a personal website, combined with my growing fondness for the computer terminal. I decided to make a [personal website that felt like a bash-like terminal](https://arekouzounian.com), and over the course of a few weeks, I succeeded! However, I soon realized that employers and non-tech savvy people would be entirely confused by my original website design. Rather than throw it all away, I opted to create a [second website, one that was much more visual and graphical](https://gui.arekouzounian.com). Further, I decided to create a third website, this time employing a REST-style API that the two other websites could draw from to sync their content. Some Flask, Apache, and JavaScript later, it worked! All three websites were up and running. 


## The Idea 
Through my work as a Teaching Assistant and Course Producer, as well as all the learning about technology I do in my free time, I realized that having a blog to catalogue thoughts and talk about personal projects/ideas would be a great idea. As I thought about the process of integrating a blog into my personal website, I realized that the typical process would be tedious; whether it was integrating some pre-made blogging framework into my existing backend, or manually creating blog posts in HTML every time I wanted to blog about something, then manually putting those files on my servers--none of those options seemed appealing to me. So, I set out to create my own blogging framework, one that could abstract all of the tediousness behind a simple client and server model that was highly flexible and easy to use. Here we are! 

## Conclusion
That brings us to the end of this blog post. I have some notes after this that are mainly for myself; just a couple questions and ideas I thought of while writing this blog post. 


### Thoughts for future implementation 
--- 
How to ensure embedded media works correctly? 
- parse the markdown file to look for relative links to media, then fetch and send to server? 
- have a CLI tool command like 'bloggen post init' that creates a directory structure that contains areas for the post, and an asset subdirectory
  - for uploading, tool tar's the folders together then sends all in one file? 
