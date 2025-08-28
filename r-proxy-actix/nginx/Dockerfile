FROM nginx:stable-alpine

COPY nginx.conf /etc/nginx/conf.d/default.conf

RUN apk add curl

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
