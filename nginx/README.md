# nginx
This folder houses the config for nginx.

This project uses an nginx container as a reverse proxy in front of the nextjs server; this lets us route traffic through standard ports like 80 and 443.

By default, only port 80 is set-up to listen for traffic, but if you want SSL, you can change the [server.conf](./server.conf) file to have access to your SSL certs by uncommenting and modifying the relevant lines.

Additionally, you will likely need to add a line to your dockerfile that copies over the relevant cert files into an accessible place within the `nginx` container.
