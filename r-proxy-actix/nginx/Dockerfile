FROM nginx:stable-alpine

COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

RUN adduser -D appuser
USER appuser

CMD ["nginx", "-g", "daemon off;"]
