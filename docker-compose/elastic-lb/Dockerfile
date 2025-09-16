# Use the HAProxy base image
FROM haproxy:3.2

# Switch to root user to install packages
USER root

# Update package lists and install curl
RUN apt-get update && apt-get install -y curl && apt-get clean

# Switch back to the default non-root user (optional, for security)
USER haproxy

# Standard start command for HAProxy
CMD ["haproxy", "-f", "/usr/local/etc/haproxy/haproxy.cfg"]
