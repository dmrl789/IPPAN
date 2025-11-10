# IPPAN Unified UI - Deployment Guide

This guide covers multiple deployment strategies for the IPPAN Unified UI static export.

## 📦 Build Output

After running `npm run build`, the static files are generated in the `./out` directory:

```
out/
├── _next/              # Next.js assets (JS, CSS)
│   ├── static/         # Static chunks
│   └── ...
├── 404.html            # 404 error page
├── index.html          # Main application
└── ...                 # Other static assets
```

## 🚀 Deployment Methods

### 1. Docker (Recommended for Production)

**Build and run with Docker:**

```bash
# Build image
docker build -t ippan-unified-ui .

# Run container
docker run -d -p 80:80 --name ippan-ui ippan-unified-ui

# With custom port
docker run -d -p 3000:80 --name ippan-ui ippan-unified-ui
```

**Using npm scripts:**

```bash
npm run docker:build
npm run docker:run
```

**With environment variables:**

```bash
docker build \
  --build-arg NEXT_PUBLIC_API_BASE_URL=https://api.ippan.network \
  --build-arg NEXT_PUBLIC_WS_URL=wss://api.ippan.network/ws \
  -t ippan-unified-ui .
```

The Docker image uses Nginx to serve the static files efficiently.

### 2. Nginx (Direct)

**Install Nginx:**

```bash
# Ubuntu/Debian
sudo apt update && sudo apt install nginx -y

# CentOS/RHEL
sudo yum install nginx -y
```

**Deploy static files:**

```bash
# Build the application
npm run build

# Copy to Nginx web root
sudo cp -r out/* /var/www/html/

# Copy Nginx configuration
sudo cp nginx.conf /etc/nginx/sites-available/ippan-ui
sudo ln -s /etc/nginx/sites-available/ippan-ui /etc/nginx/sites-enabled/

# Test and reload
sudo nginx -t
sudo systemctl reload nginx
```

**Nginx configuration is provided in `nginx.conf` with:**
- Gzip compression
- Security headers
- Static asset caching
- SPA routing support
- Health check endpoint

### 3. Apache

**Install Apache:**

```bash
# Ubuntu/Debian
sudo apt install apache2 -y

# CentOS/RHEL
sudo yum install httpd -y
```

**Deploy and configure:**

```bash
# Build
npm run build

# Copy files
sudo cp -r out/* /var/www/html/

# Enable mod_rewrite
sudo a2enmod rewrite
```

**Create `.htaccess` in `/var/www/html/`:**

```apache
<IfModule mod_rewrite.c>
  RewriteEngine On
  RewriteBase /
  
  # Redirect to HTTPS (optional)
  # RewriteCond %{HTTPS} off
  # RewriteRule ^(.*)$ https://%{HTTP_HOST}%{REQUEST_URI} [L,R=301]
  
  # Handle SPA routing
  RewriteRule ^index\.html$ - [L]
  RewriteCond %{REQUEST_FILENAME} !-f
  RewriteCond %{REQUEST_FILENAME} !-d
  RewriteRule . /index.html [L]
</IfModule>

# Enable Gzip
<IfModule mod_deflate.c>
  AddOutputFilterByType DEFLATE text/html text/plain text/xml text/css text/javascript application/javascript
</IfModule>

# Cache static assets
<IfModule mod_expires.c>
  ExpiresActive On
  ExpiresByType image/jpg "access plus 1 year"
  ExpiresByType image/jpeg "access plus 1 year"
  ExpiresByType image/gif "access plus 1 year"
  ExpiresByType image/png "access plus 1 year"
  ExpiresByType text/css "access plus 1 year"
  ExpiresByType application/javascript "access plus 1 year"
</IfModule>
```

**Restart Apache:**

```bash
sudo systemctl restart apache2  # Ubuntu/Debian
sudo systemctl restart httpd    # CentOS/RHEL
```

### 4. Static Hosting Services

#### Vercel

```bash
# Install Vercel CLI
npm i -g vercel

# Deploy
vercel deploy --prod

# Or link to Git repository for automatic deployments
```

**vercel.json:**

```json
{
  "buildCommand": "npm run build",
  "outputDirectory": "out",
  "cleanUrls": true,
  "trailingSlash": false
}
```

#### Netlify

```bash
# Install Netlify CLI
npm i -g netlify-cli

# Deploy
netlify deploy --prod --dir=out
```

**netlify.toml:**

```toml
[build]
  command = "npm run build"
  publish = "out"

[[redirects]]
  from = "/*"
  to = "/index.html"
  status = 200
```

#### AWS S3 + CloudFront

```bash
# Build
npm run build

# Sync to S3
aws s3 sync out/ s3://your-bucket-name/ --delete

# Invalidate CloudFront cache
aws cloudfront create-invalidation \
  --distribution-id YOUR_DISTRIBUTION_ID \
  --paths "/*"
```

**S3 Bucket Policy (public read):**

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "PublicReadGetObject",
      "Effect": "Allow",
      "Principal": "*",
      "Action": "s3:GetObject",
      "Resource": "arn:aws:s3:::your-bucket-name/*"
    }
  ]
}
```

#### GitHub Pages

```bash
# Build
npm run build

# Install gh-pages
npm install -D gh-pages

# Add to package.json scripts:
# "deploy": "gh-pages -d out"

# Deploy
npm run deploy
```

**Note:** GitHub Pages may require `basePath` in `next.config.js` if deployed to a repository subdirectory:

```js
module.exports = {
  basePath: '/repo-name',
  // ...
}
```

### 5. Local Testing

**Using the built-in serve script:**

```bash
# Build first
npm run build

# Serve locally on port 3000
npm run serve
```

**Using Python:**

```bash
cd out
python3 -m http.server 8000
```

**Using Node.js http-server:**

```bash
npx http-server out -p 8000
```

## 🔧 Environment Configuration

### Build-time Variables

Environment variables are baked into the static build at build time. To change them:

1. Update `.env.production` or set environment variables
2. Rebuild: `npm run build`
3. Redeploy the new `out/` directory

**Example:**

```bash
NEXT_PUBLIC_API_BASE_URL=https://api.ippan.network \
NEXT_PUBLIC_WS_URL=wss://api.ippan.network/ws \
npm run build
```

### Multiple Environments

**Option 1: Multiple .env files**

```bash
# Build for staging
cp .env.staging .env.production
npm run build

# Build for production
cp .env.prod .env.production
npm run build
```

**Option 2: Environment-specific builds**

```bash
# Staging
NEXT_PUBLIC_API_BASE_URL=https://staging-api.ippan.network npm run build
mv out out-staging

# Production
NEXT_PUBLIC_API_BASE_URL=https://api.ippan.network npm run build
mv out out-production
```

## 🔐 Security Considerations

### 1. HTTPS/TLS

Always use HTTPS in production:

```nginx
server {
    listen 443 ssl http2;
    server_name ui.ippan.network;
    
    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;
    
    # ... rest of config
}
```

### 2. Security Headers

Already configured in `nginx.conf`:

- `X-Frame-Options: SAMEORIGIN`
- `X-Content-Type-Options: nosniff`
- `X-XSS-Protection: 1; mode=block`
- `Referrer-Policy: no-referrer-when-downgrade`

### 3. CORS Configuration

The UI makes requests to backend APIs. Ensure backend CORS is configured:

```
Access-Control-Allow-Origin: https://ui.ippan.network
Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS
Access-Control-Allow-Headers: Content-Type, Authorization
```

### 4. API Keys

Never expose sensitive API keys in the UI. All `NEXT_PUBLIC_*` variables are visible to users.

## 📊 Monitoring

### Health Check

The Nginx configuration includes a health check endpoint:

```bash
curl http://your-domain/health
# Response: healthy
```

### Logs

**Docker:**

```bash
docker logs ippan-ui -f
```

**Nginx:**

```bash
tail -f /var/log/nginx/access.log
tail -f /var/log/nginx/error.log
```

## 🔄 CI/CD Integration

### GitHub Actions

`.github/workflows/deploy-ui.yml`:

```yaml
name: Deploy UI

on:
  push:
    branches: [main]
    paths:
      - 'apps/unified-ui/**'

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '20'
          
      - name: Install dependencies
        working-directory: apps/unified-ui
        run: npm ci
        
      - name: Build
        working-directory: apps/unified-ui
        env:
          NEXT_PUBLIC_API_BASE_URL: ${{ secrets.API_BASE_URL }}
          NEXT_PUBLIC_WS_URL: ${{ secrets.WS_URL }}
        run: npm run build
        
      - name: Deploy to S3
        uses: jakejarvis/s3-sync-action@master
        with:
          args: --delete
        env:
          AWS_S3_BUCKET: ${{ secrets.AWS_S3_BUCKET }}
          AWS_ACCESS_KEY_ID: ${{ secrets.AWS_ACCESS_KEY_ID }}
          AWS_SECRET_ACCESS_KEY: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
          SOURCE_DIR: 'apps/unified-ui/out'
```

### GitLab CI

`.gitlab-ci.yml`:

```yaml
deploy-ui:
  stage: deploy
  image: node:20-alpine
  script:
    - cd apps/unified-ui
    - npm ci
    - npm run build
    - # Deploy commands here
  only:
    - main
  changes:
    - apps/unified-ui/**/*
```

## 🐛 Troubleshooting

### Issue: Blank page after deployment

**Cause:** Incorrect asset paths or base path

**Solution:**
1. Check browser console for 404 errors
2. Verify `NEXT_PUBLIC_ASSET_PREFIX` if using CDN
3. Check `basePath` in `next.config.js` for subdirectory deployments

### Issue: API requests failing

**Cause:** CORS or incorrect API URL

**Solution:**
1. Verify `NEXT_PUBLIC_API_BASE_URL` in build
2. Check backend CORS configuration
3. Use browser dev tools Network tab to inspect requests

### Issue: 404 on page refresh

**Cause:** Server not configured for SPA routing

**Solution:**
- Nginx: Use provided `nginx.conf` with `try_files`
- Apache: Add `.htaccess` with rewrite rules
- Static hosts: Configure redirects to `index.html`

### Issue: Environment variables not updating

**Cause:** Variables are baked at build time

**Solution:**
1. Update environment variables
2. **Rebuild:** `npm run build`
3. Redeploy the new build

## 📝 Checklist

Before deploying to production:

- [ ] Set production API URLs in `.env.production`
- [ ] Test build locally: `npm run build && npm run serve`
- [ ] Verify API connectivity from built version
- [ ] Test WebSocket connections
- [ ] Configure HTTPS/SSL certificates
- [ ] Set up monitoring and logging
- [ ] Configure CDN (optional)
- [ ] Test 404 and error pages
- [ ] Verify security headers
- [ ] Set up automated backups
- [ ] Document rollback procedure

---

**Need help?** Check the main [README.md](./README.md) or contact the maintainers.
