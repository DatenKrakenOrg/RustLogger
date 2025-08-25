FROM nginx:stable-alpine

RUN adduser -D appuser
USER appuser

COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
