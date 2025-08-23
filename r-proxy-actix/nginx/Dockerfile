RUN adduser -D appuser
USER appuserUN adduser -D appuser
USER appuser

FROM nginx:stable-alpine

COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
