# 🚀 IPPAN Full-Stack Deployment Guide

## 🌐 **Complete System Architecture**

Your IPPAN deployment includes:

- **Blockchain Nodes** – 2 IPPAN validator nodes (consensus)
- **Load Balancer** – Nginx for distributing requests
- **API Gateway** – Unified REST and WebSocket access across nodes

---

## 📋 **Deployment Options**

### **Option 1 – Full-Stack Docker Deployment (Recommended)**

#### **Server 1 – Primary (188.245.97.41)**

```bash
git clone <your-repo-url>
cd ippan
docker compose -f deploy/docker-compose.full-stack.yml up -d
docker compose -f deploy/docker-compose.full-stack.yml ps
