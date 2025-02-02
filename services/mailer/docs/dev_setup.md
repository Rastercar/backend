# Development setup

To develop/run this service you will need Jaeger and rabbitmq, like in this example docker-compose file.

```yaml
version: '3.7'

services:
  rabbitmq:
    container_name: rastercar-rmq
    image: rabbitmq:3.10.1-management
    ports:
      - 5672:5672
      - 15672:15672

  # see: https://www.jaegertracing.io/docs/2.0/getting-started/
  jaeger:
    container_name: rastercar-jaeger
    image: 'jaegertracing/jaeger:2.0.0'
    environment:
      - COLLECTOR_OTLP_ENABLED=true
    ports:
      - '5778:5778' # Configuration
      - '16686:16686' # Query UI
      - '4317:4317' # OTLP gRPC
      - '4318:4318' # OTLP HTTP
      - '14250:14250' # gRPC for collector
      - '14268:14268' # HTTP for collector
      - '9411:9411' # Zipkin
    command:
      - --set
      - receivers.otlp.protocols.http.endpoint=0.0.0.0:4318
      - --set
      - receivers.otlp.protocols.grpc.endpoint=0.0.0.0:4317
```

## SES setup

First create a SES identity to use as the default email sender address

![sender](./imgs/create-sender.png "sender")

after verifying the identity email address, you can use it as the default sender
by setting the `APP_DEFAULT_EMAIL_SENDER` env var

Now we need to create a configuration set to track email events using SNS and SES, following steps:

- create a SES configuration set
- create a SNS event topic with the standard type
- on your SES configuration set, add a event destination to the created SNS topic
- create a HTTPS subscription pointing to the URL of where you pretend to host this service
- finally run this service with the env var `AWS_SES_TRACKING_CONFIG_SET` set to the name of the cfg set you created


## Routing SNS events to your local machine

When developing, its handy to route SNS requests directly to your machine, to do this you need to give it
a public url/ip, one easy way is using [NGROK](https://ngrok.com/). after installing just run

```sh
ngrok http 3005
```

now create a SNS subscription pointing to the generated url + /ses-events, eg:

![subscription](./imgs/create-subscription.png "subscription")

after sending a confirmation request, the link will be printed to stdout

```sh
[WEB] SNS subscription confirmation link: https://sns.us-east-1.amazonaws.com/?Action=ConfirmSubscription&TopicArn=arn:aws:sns:...&Token=...
```

Just click the link to activate the subscription. Now every email with `"enableTracking": true` should publish its events to SNS, which
will publish the events to the HTTPS endpoint to your PC.

## Available Environment variables

|           name                    |                                    meaning                         | example                           |
|-----------------------------------|--------------------------------------------------------------------|-----------------------------------|
| APP_DEBUG                         | debug mode, if true will log to debug info to stdout               | false                             |
| APP_DEFAULT_EMAIL_SENDER          | default email address to be used as the sender                     | <no-reply@your-company.com>       |
| RMQ_URI                           | rabbitmq url                                                       | amqp://guest:guest@localhost:5672 |
| RMQ_QUEUE                         | name of the rabbitmq queue to listen for messages                  | mailer_queue                      |
| RMQ_CONSUMER_TAG                  | name of the consumer tag for the queue consumer                    | mailer_queue_consumer             |
| RMQ_EMAIL_EVENTS_EXCHANGE         | name for the exchange to publish email events on                   | mailer_events                     |
| AWS_REGION                        |                                                                    | us-east-1                         |
| AWS_SES_TRACKING_CONFIG_SET       | name of the SES configuration set to use for email tracking        | track-all-events                  |
| AWS_SES_MAX_EMAILS_PER_SECOND     | limit for ops/s for the SES send email operation for your account  | 1                                 |
| AWS_SNS_TRACKING_SUBSCRIPTION_ARN | AWS ARN for the SNS subscription for the email tracking config set | arn:123...                        |
| TRACER_SERVICE_NAME               | name of the service to jaeger                                      | mailer                            |
| HTTP_PORT                         | HTTP port to listen on for SNS events                              | 3005                              |
