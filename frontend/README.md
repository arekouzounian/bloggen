# bloggen Frontend
This repository will house all the files related to the bloggen web frontend. 

Currently, the frontend is intended to be rendered using [The Yew Framework](https://github.com/yewstack/yew). The general idea is to have the frontend draw dynamically from inner HTML files (parsed from markdown using the CLI tool) to populate pages (posts).

As of now, this frontend will heavily draw from the code in [The Yew Router Template](https://github.com/yewstack/yew/tree/master/examples/router)

The end goal for the frontend is to be able to be served via Apache, and to have everything packaged neatly within a Docker container.
