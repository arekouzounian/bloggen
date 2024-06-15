# this is based on [an amazing blog post](https://steveholgado.com/nginx-for-nextjs/)

FROM nginx:alpine

# Remove any existing config files
RUN rm /etc/nginx/conf.d/*

# Copy config files
# *.conf files in "conf.d/" dir get included in main config
COPY ./server.conf /etc/nginx/conf.d/default.conf

# Expose the listening port
EXPOSE 80

# Launch NGINX
CMD [ "nginx", "-g", "daemon off;" ]