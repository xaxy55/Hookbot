# Cloud Deployment Setup

Hookbot runs on **Google Cloud e2-micro** (free forever) + **Cloudflare Pages** (free forever).

## 1. Google Cloud Setup

### Create project & enable APIs

```bash
# Install gcloud CLI: https://cloud.google.com/sdk/docs/install
gcloud auth login

# Create project (or use an existing one)
gcloud projects create hookbot-prod --name="Hookbot"
gcloud config set project hookbot-prod

# Enable required APIs
gcloud services enable compute.googleapis.com
gcloud services enable artifactregistry.googleapis.com

# Enable billing (required even for free tier)
# Go to: https://console.cloud.google.com/billing
# Link your project to a billing account (you won't be charged for free-tier resources)
```

### Create a service account for CI/CD

```bash
# Create service account
gcloud iam service-accounts create hookbot-deploy \
  --display-name="Hookbot CI/CD"

# Grant required roles
PROJECT_ID=$(gcloud config get-value project)

gcloud projects add-iam-policy-binding $PROJECT_ID \
  --member="serviceAccount:hookbot-deploy@${PROJECT_ID}.iam.gserviceaccount.com" \
  --role="roles/compute.admin"

gcloud projects add-iam-policy-binding $PROJECT_ID \
  --member="serviceAccount:hookbot-deploy@${PROJECT_ID}.iam.gserviceaccount.com" \
  --role="roles/artifactregistry.admin"

gcloud projects add-iam-policy-binding $PROJECT_ID \
  --member="serviceAccount:hookbot-deploy@${PROJECT_ID}.iam.gserviceaccount.com" \
  --role="roles/iam.serviceAccountUser"

# Create and download key
gcloud iam service-accounts keys create gcp-sa-key.json \
  --iam-account="hookbot-deploy@${PROJECT_ID}.iam.gserviceaccount.com"

echo "Key saved to gcp-sa-key.json — keep this safe, don't commit it!"
```

## 2. Cloudflare Setup

1. Create account at https://dash.cloudflare.com/sign-up
2. Go to **My Profile → API Tokens → Create Token**
3. Use the **Edit Cloudflare Workers** template (covers Pages too)
4. Copy the token
5. Your Account ID is on the right sidebar of the dashboard overview page

## 3. Set GitHub Secrets

The easiest way — run from the repo root:

```bash
make cloud-secrets
```

This interactively prompts for all values and sets them via `gh secret set`.

Or set them manually:

```bash
gh secret set GCP_PROJECT_ID --body "hookbot-prod"
gh secret set GCP_SA_KEY < gcp-sa-key.json
gh secret set CLOUDFLARE_API_TOKEN --body "your-cf-token"
gh secret set CLOUDFLARE_ACCOUNT_ID --body "your-cf-account-id"
# API_BASE_URL is set after first deploy (once you know the static IP)
```

## 4. First Deploy

```bash
# Provision infrastructure
cd infra
terraform init
terraform apply \
  -var="gcp_project_id=hookbot-prod" \
  -var="cloudflare_api_token=YOUR_TOKEN" \
  -var="cloudflare_account_id=YOUR_ACCOUNT_ID"

# Note the static IP from output
terraform output server_ip

# Now set the API_BASE_URL secret
gh secret set API_BASE_URL --body "http://$(terraform output -raw server_ip)"
```

After this, pushing to `main` triggers automatic deploys:
- `server/**` changes → builds Docker image, deploys to GCE
- `web/**` changes → builds React, deploys to Cloudflare Pages
- `infra/*.tf` changes → Terraform plan (on PR) / apply (on merge)

## 5. Verify

```bash
# Check server is running
curl http://$(terraform output -raw server_ip)/api/health

# Check frontend
echo "Frontend: $(terraform output -raw cloudflare_pages_url)"
```

## Free Tier Limits

| Resource | Free Allowance |
|----------|---------------|
| GCE e2-micro | 1 VM in us-central1/us-west1/us-east1 |
| Boot disk | 30 GB standard (not SSD) |
| Static IP | Free while attached to running VM |
| Egress | 1 GB/month (US regions) |
| Artifact Registry | 500 MB storage |
| Cloudflare Pages | Unlimited bandwidth, 500 builds/month |

## Note on Device Polling

The device poller reaches `192.168.x.x` addresses which are unreachable from the cloud. Polling will silently fail. This is expected — the cloud deployment is for the web UI and API, not LAN device control.
