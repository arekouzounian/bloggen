# Version 2
---
## Overview - v1
At a high level, the bloggen architecture works as follows: 
- A Go client that handles the lifecycle of a post locally
- A Rust server that handles the uploading and saving of posts
- A Next.js frontend that handles the retrieving of posts and displaying them as a webpage.

Here are issues with how the the current architecture is implemented:
- The client interface could be improved; currently, the user has to use two separate commands and manage a directory structure for each post. Managing a directory structure isn't inherently bad, but the main issue is that the local directory structure doesn't have any capability to sync with the server. 
 - Updates happen by re-uploading the whole post, which is somewhat inefficient, and edge cases arise with improper uploads to the server or an inconsistent state.
- The server implements some operations of SFTP, but not really that many of them (I got lazy). More sophisticated post management could be performed over SFTP, but this currently isn't implemented. 
- The way that posts are stored and the way that the frontend retrieves them is frankly incredibly naive. When a client uploads a post, the post files are converted in-place to HTML and then the whole directory is just SFTP copied onto the server. Then, the next.js frontend just looks for those files locally, reads the HTML, and just loads it raw into the page. This is pretty awful.

## Overview - v2
Version 2 aims to address these concerns by implementing the following changes:
- Posts will be converted by the client into JSON. The markdown will be parsed into a JSON payload, and uploaded to the server as such. This means that metadata can be included on the same payload, instead of being stored as a separate file. 
  - assets will probably still be separate files (? not entirely decided on this yet. If they're in the JSON payload we probably have to b64 encode which might inflate payload size; can we perform compresson on the payload?)
  - another thought; maybe client resolves web urls and downloads the assets before uploading, and includes hash of the asset in the post payload. Then server can store hashes to deduplicate assets
- A change to the client interface will be made. The current prevailing idea is to use a FUSE driver to fully abstract server interaction behind a filesystem-like interface. More thought needs to be given to this.
- The server will use an actual database (finally) instead of storing straight files like a dumb idiot. The database will probably be on the same host because this is a blogging platform and isn't designed for *planet-scale*.
- The frontend also needs to get a rework given that it won't just be fetching files. Since data is fetched more dynamically instead of raw HTML being injected into the page, we can allow the user more options in styling their blog posts without having to dive into a nested tangle of tailwind and raw CSS.

## Things that are sticking around in v2
The main things sticking around in v2 are the design goals/philosophies. The primary emphasis of this software is ease-of-use and simple security with minimal setup. 

Bloggen is still designed for the same use case: a medium/small blog, with one administrator, that has SSH access to a server. With the proposed architectural changes, it's possible that we can relax the 'only one admin' constraint to allow for multiple people to upload to the blog, but that isn't necessarily in the scope of the v2 changes, and may be relegated more fully to a future version.
