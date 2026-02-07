#!/bin/bash
# LocalStack initialization script
# Creates SQS queues for PaperForge

echo "Initializing PaperForge SQS queues..."

# Create ingestion queue
awslocal sqs create-queue --queue-name paperforge-ingestion

# Create embedding queue
awslocal sqs create-queue --queue-name paperforge-embedding

# Create DLQ for failed messages
awslocal sqs create-queue --queue-name paperforge-dlq

# Set up redrive policy (move to DLQ after 3 failures)
INGESTION_ARN=$(awslocal sqs get-queue-attributes --queue-url http://localhost:4566/000000000000/paperforge-ingestion --attribute-names QueueArn --query 'Attributes.QueueArn' --output text)
DLQ_ARN=$(awslocal sqs get-queue-attributes --queue-url http://localhost:4566/000000000000/paperforge-dlq --attribute-names QueueArn --query 'Attributes.QueueArn' --output text)

awslocal sqs set-queue-attributes \
  --queue-url http://localhost:4566/000000000000/paperforge-ingestion \
  --attributes "{\"RedrivePolicy\":\"{\\\"deadLetterTargetArn\\\":\\\"$DLQ_ARN\\\",\\\"maxReceiveCount\\\":3}\"}"

awslocal sqs set-queue-attributes \
  --queue-url http://localhost:4566/000000000000/paperforge-embedding \
  --attributes "{\"RedrivePolicy\":\"{\\\"deadLetterTargetArn\\\":\\\"$DLQ_ARN\\\",\\\"maxReceiveCount\\\":3}\"}"

echo "SQS queues created:"
awslocal sqs list-queues

echo "LocalStack initialization complete!"
