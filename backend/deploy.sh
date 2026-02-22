#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   AWS_ACCOUNT_ID=123456789012 ./deploy.sh [image_tag]
# Optional env vars:
#   AWS_REGION (default: ap-northeast-1)
#   REPO_NAME (default: backend-lambda)
#
# IMPORTANT:
# This build runs on an ARM64 machine and produces an ARM64 Lambda container image.
# When creating/updating the Lambda function, select Architecture = ARM64.

AWS_ACCOUNT_ID="${AWS_ACCOUNT_ID:-}"
AWS_REGION="${AWS_REGION:-ap-northeast-1}"
REPO_NAME="${REPO_NAME:-task-app-backend-lambda}"
IMAGE_TAG="${1:-latest}"

if [[ -z "$AWS_ACCOUNT_ID" ]]; then
  echo "Error: AWS_ACCOUNT_ID is required."
  echo "Example: AWS_ACCOUNT_ID=123456789012 ./deploy.sh"
  exit 1
fi

ECR_REGISTRY="${AWS_ACCOUNT_ID}.dkr.ecr.${AWS_REGION}.amazonaws.com"
ECR_REPO_URI="${ECR_REGISTRY}/${REPO_NAME}"

echo "Checking ECR repository: ${REPO_NAME} (${AWS_REGION})"
if ! aws ecr describe-repositories --repository-names "$REPO_NAME" --region "$AWS_REGION" >/dev/null 2>&1; then
  echo "ECR repository '${REPO_NAME}' does not exist in region '${AWS_REGION}'."
  echo "Create it with:"
  echo "  aws ecr create-repository --repository-name ${REPO_NAME} --region ${AWS_REGION}"
  exit 1
fi

echo "Authenticating Docker to ECR..."
aws ecr get-login-password --region "$AWS_REGION" | docker login --username AWS --password-stdin "$ECR_REGISTRY"

echo "Building Docker image from Dockerfile.lambda..."
# Note: BUILDX_NO_DEFAULT_ATTESTATIONS=1 and --provenance=false ensure Lambda compatibility.
# We use --platform linux/arm64 to match the target architecture exactly.
BUILDX_NO_DEFAULT_ATTESTATIONS=1 docker buildx build \
  --platform linux/arm64 \
  --provenance=false \
  --output type=docker \
  -f Dockerfile.lambda \
  -t "${REPO_NAME}:${IMAGE_TAG}" .

echo "Tagging image..."
docker tag "${REPO_NAME}:${IMAGE_TAG}" "${ECR_REPO_URI}:${IMAGE_TAG}"

echo "Pushing image to ECR..."
docker push "${ECR_REPO_URI}:${IMAGE_TAG}"

echo "Done. Pushed: ${ECR_REPO_URI}:${IMAGE_TAG}"
