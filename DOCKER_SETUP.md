# Docker Setup for Multi-Type Logging System

## Overview
This Docker Compose setup runs the complete logging pipeline with support for multiple message types and configurable log structures.

## Architecture Components

### 1. Configuration Setup
- **Direct Mount**: `message_types.toml` mounted directly to each service
- **Read-Only**: Configuration file mounted as `:ro` for safety

### 2. Log Generation
- **log_gen**: Generates multiple CSV files based on message types
- **Environment**: Uses `/etc/config/message_types.toml` for configuration
- **Output**: Creates CSV files in `/etc/logs/` volume

### 3. Log Sending
- **log_sender**: Randomly selects a message type and sends logs
- **Environment Variables**: 
  - `LOGS_DIRECTORY=/etc/logs/`
  - `CONFIG_PATH=/etc/config/message_types.toml`
  - `ENDPOINT=http://log-forwarding-lb:80/send_log`

### 4. API Services
- **log-forwarding-api-[1-3]**: Handle dynamic message type parsing
- **Environment Variables**:
  - `CONFIG_PATH=/etc/config/message_types.toml`
  - Dynamic Elasticsearch index creation based on message types

### 5. Infrastructure
- **HAProxy**: Load balancer for Elasticsearch cluster
- **Elasticsearch Cluster**: 4-node cluster (es01-es04)
- **Kibana**: Visualization and management
- **Metricbeat**: System monitoring

## Updated Environment Variables

### Key Changes in `.env`:
```bash
# New multi-type configuration
LOGS_DIRECTORY="/etc/logs/"
CONFIG_PATH="/etc/config/message_types.toml"

# Removed INDEX_NAME (now dynamic per message type)
# Dynamic indices: logs-iot-sensors, logs-kafka, logs-timescaledb, etc.
```

## Message Types Supported

1. **iot_sensor** → `logs-iot-sensors` index
2. **system_metrics** → `logs-system-metrics` index  
3. **timescaledb** → `logs-timescaledb` index
4. **kafka** → `logs-kafka` index
5. **application_logs** → `logs-application` index

## Running the System

### Start the complete stack:
```bash
cd docker-compose
docker compose up -d
```

### Monitor logs:
```bash
# View log generation
docker compose logs -f log_gen

# View log sending
docker compose logs -f log_sender

# View API processing
docker compose logs -f log-forwarding-api-1
```

### Access Services:
- **Kibana**: http://localhost:5601
- **Elasticsearch**: http://localhost:9200 (via HAProxy)
- **HAProxy Stats**: http://localhost:8404
- **API Load Balancer**: http://localhost:8080

## Volume Structure

```
volumes/
├── logs/                      # Shared logs volume
│   ├── iot_sensor.csv         # Generated CSV files
│   ├── system_metrics.csv
│   ├── timescaledb.csv
│   ├── kafka.csv
│   └── application_logs.csv
└── [elasticsearch volumes...]

host/
└── message_types.toml         # Configuration file (directly mounted)
```

## Key Features

✅ **Multi-Type Support**: 5 different message types with realistic data  
✅ **Dynamic Indices**: Elasticsearch indices created automatically per type  
✅ **Contextual Logic**: Logs have realistic relationships (temp→humidity, CPU→memory)  
✅ **Random Selection**: log_sender randomly picks a message type each run  
✅ **Configuration-Driven**: Easy to add new message types via TOML config  
✅ **Docker-Ready**: Complete containerized deployment  

## Troubleshooting

### Configuration Issues:
```bash
# Check if config is properly mounted
docker compose exec log_gen ls -la /etc/config/message_types.toml

# Verify message types configuration
docker compose exec log_gen cat /etc/config/message_types.toml

# Test configuration parsing
docker compose exec log-forwarding-api-1 ls -la /etc/config/message_types.toml
```

### API Issues:
```bash
# Test API directly
curl -X POST http://localhost:8080/send_log \
  -H "Content-Type: application/json" \
  -d '{"message_type": "iot_sensor", "csv_line": "2025-08-27T14:30:22Z,INFO,25.0,0.5,sensor_001,office"}'
```

### Elasticsearch Issues:
```bash
# Check cluster health
curl -u elastic:123456 http://localhost:9200/_cluster/health

# List indices
curl -u elastic:123456 http://localhost:9200/_cat/indices
```

## Migration Notes

This setup replaces the single-type logging with:
- Multiple message types
- Dynamic index creation
- Configuration-driven field parsing
- Realistic contextual data generation

The system maintains backward compatibility while adding significant flexibility for different log types.