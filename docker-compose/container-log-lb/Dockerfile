FROM nginx:stable-alpine

COPY nginx.conf etc/nginx/nginx.conf

RUN apk add curl

Expose 5050/udp
Expose 80/tcp

CMD ["nginx", "-g", "daemon off;"]
