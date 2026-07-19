#!/bin/sh
# gcp-billing-setup.sh — deploy the HARD billing cutoff (layer 3). One-shot.
# Wires:  Budget (= your remaining credit) --> Pub/Sub --> Cloud Function that DISABLES billing.
# Result: when actual spend reaches the budget, billing detaches and everything stops. No card charge.
#
# Usage:  bash gcp-billing-setup.sh <BUDGET_DOLLARS>
#   e.g.  bash gcp-billing-setup.sh 250      # set to your remaining credit, or a safe buffer below it
#
# NOTE: steps 4 + 5 touch the BILLING ACCOUNT (need the Billing Account Administrator role, which
# you have after upgrading). If any billing step is blocked in this shell, run that one command in
# the Cloud Shell / console — the function code + everything else is already in place.
set -e
GCLOUD="/c/Program Files (x86)/Google/Cloud SDK/google-cloud-sdk/bin/gcloud.cmd"
PROJECT=project-850d63d4-bf88-46df-8cb
BILLING=01D642-C94000-DF9245
REGION=us-central1
TOPIC=billing-guard
SA_NAME=billing-guard
SA_EMAIL="$SA_NAME@$PROJECT.iam.gserviceaccount.com"
BUDGET="${1:?pass the budget in dollars, e.g. bash gcp-billing-setup.sh 250}"

echo "[1/5] enabling APIs..."
"$GCLOUD" services enable cloudbilling.googleapis.com billingbudgets.googleapis.com \
  cloudfunctions.googleapis.com run.googleapis.com cloudbuild.googleapis.com pubsub.googleapis.com \
  --project="$PROJECT"

echo "[2/5] Pub/Sub topic + dedicated service account..."
"$GCLOUD" pubsub topics create "$TOPIC" --project="$PROJECT" 2>/dev/null || echo "  (topic exists)"
"$GCLOUD" iam service-accounts create "$SA_NAME" --project="$PROJECT" \
  --display-name="Billing hard-cutoff function" 2>/dev/null || echo "  (SA exists)"

echo "[3/5] deploying the Cloud Function (gen2, python)..."
"$GCLOUD" functions deploy billing-guard \
  --gen2 --runtime=python312 --region="$REGION" --project="$PROJECT" \
  --source=gcp-billing-guard --entry-point=stop_billing \
  --trigger-topic="$TOPIC" --service-account="$SA_EMAIL" \
  --set-env-vars="PROJECT_ID=$PROJECT"

echo "[4/5] *** BILLING STEP *** grant the function SA rights to disable billing..."
"$GCLOUD" billing accounts add-iam-policy-binding "$BILLING" \
  --member="serviceAccount:$SA_EMAIL" --role="roles/billing.admin"

echo "[5/5] *** BILLING STEP *** create the budget (= \$$BUDGET) wired to the topic..."
"$GCLOUD" billing budgets create --billing-account="$BILLING" \
  --display-name="bastion-hard-cap" \
  --budget-amount="${BUDGET}USD" \
  --all-updates-rule-pubsub-topic="projects/$PROJECT/topics/$TOPIC" \
  --threshold-rule=percent=0.5 --threshold-rule=percent=0.9 --threshold-rule=percent=1.0

echo "=== DONE. At \$$BUDGET spend, billing auto-disables. Re-enable in console to thaw. ==="
echo "TEST (safe): publish a fake over-budget event ->"
echo "  $GCLOUD pubsub topics publish $TOPIC --project=$PROJECT --message='{\"costAmount\":999,\"budgetAmount\":$BUDGET}'"
echo "  then check:  $GCLOUD billing projects describe $PROJECT   (billingEnabled should flip to false)"
