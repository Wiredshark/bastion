"""Cloud Function: hard billing cutoff. Wired to a Cloud Billing Budget via Pub/Sub.
When actual spend crosses the budget amount, it DETACHES the billing account from the
project, which stops all billable resources (VMs, etc.). This is the "cut off when the
credits are exhausted" backstop — set the budget amount to your remaining credit (or a
safe buffer below it). Re-enable billing in the console to thaw the project afterward.

Deploy: see ../gcp-billing-setup.sh
"""
import base64
import json
import os

from googleapiclient import discovery

PROJECT_ID = os.environ["PROJECT_ID"]
PROJECT_NAME = f"projects/{PROJECT_ID}"


def stop_billing(event, context):
    # Budget notifications arrive as base64-encoded JSON on the Pub/Sub message.
    payload = json.loads(base64.b64decode(event["data"]).decode("utf-8"))
    cost = float(payload.get("costAmount", 0))
    budget = float(payload.get("budgetAmount", 0))
    print(f"budget notification: spend={cost} budget={budget}")

    if cost < budget:
        print("under budget — no action")
        return

    billing = discovery.build("cloudbilling", "v1", cache_discovery=False)
    info = billing.projects().getBillingInfo(name=PROJECT_NAME).execute()
    if not info.get("billingEnabled"):
        print("billing already disabled — nothing to do")
        return

    # Detaching the billing account (empty billingAccountName) disables billing → all
    # billable resources stop. This is the hard cutoff.
    billing.projects().updateBillingInfo(
        name=PROJECT_NAME, body={"billingAccountName": ""}
    ).execute()
    print(f"*** BILLING DISABLED on {PROJECT_ID} (spend {cost} >= budget {budget}) ***")
