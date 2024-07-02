# Bloggen frontend 
This is a [next.js](https://nextjs.org) frontend created using `create-next-app`.

Similarly to the server, if following the setup instructions, there should be no need to configure anything manually--you can simply set things up with docker compose, and the frontend should be hosted through port 80 (or port 443, if you additionally add SSL). 

This project uses tailwind for its styles, but many of the actual blog post stylings occur in `app/globals.css`, in plain CSS. This may be subject to change, as the way blog posts are currently converted and propagated is less than ideal, but as long as it remains this way, you are able to modify much of the blog post styling methods in this file, if you'd like. 


