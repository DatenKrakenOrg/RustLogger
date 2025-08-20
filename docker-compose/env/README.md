# Environment Configuration

This directory contains service-specific environment configuration files for the Docker Compose setup.

## Structure

```
env/
├── elasticsearch.env     # Elasticsearch cluster configuration
├── api.env              # API services configuration
├── logging.env          # Log generator/sender configuration
├── *.env.example        # Template files (safe to commit)
└── README.md           # This file
```

## Usage

1. Copy example files to create your environment configs:
   ```bash
   cp elasticsearch.env.example elasticsearch.env
   cp api.env.example api.env
   cp logging.env.example logging.env
   ```

2. Edit the `.env` files with your specific values:
   - **elasticsearch.env**: Set passwords, memory limits, cluster settings
   - **api.env**: Configure API endpoints and deployment settings
   - **logging.env**: Set log file paths and processing options

3. Start the services:
   ```bash
   docker-compose up -d
   ```

## Security Notes

- The actual `.env` files are ignored by git (see `.gitignore`)
- Only the `.env.example` template files are committed to version control
- Never commit files containing real passwords or sensitive data

## Service Mapping

| Service | Environment Files Used |
|---------|----------------------|
| elasticsetup | elasticsearch.env |
| es01, es02, es03 | elasticsearch.env |
| kibana | elasticsearch.env (for passwords) |
| log-forwarding-api-* | api.env + elasticsearch.env |
| log_sender | logging.env |
| log_gen | (no env file needed) |