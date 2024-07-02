# Bloggen server 

This is an SFTP server running atop SSH. It can be configured using an input json file, whose fields are documented in `src/config.rs`. 


Though the server can be configured manually and run singularly, its desired usage is to be set up as a docker container and then bundled with `docker compose`. To this end, there are the `docker.json`, `Dockerfile`, and `server.yaml` files that are intended to handle all configuration.


If following all the setup instructions, the server should be 'plug-n-play', and you shouldn't deal with any other configurations. 
