# Decoder

A simple micro service for receiving TCP/UDP connections from vehicle trackers and decoding their packets, sending the decoded and parsed 
messages in JSON format to a rabbitmq exchange.

## Tracker configuration

The first thing you need is to get a public IP, chances are you will need to learn about [port forwarding](https://en.wikipedia.org/wiki/Port_forwarding).
if you want to easily debug connections that install [ngrok](https://ngrok.com/) and run `ngrok tcp <port_you_configured_on_port_forwarding>`

To confirm your ip address is open to the world send a tcp packet to your ip (or the ngrok ip if you're using it) and check for any logs.
Now you only need to configure your tracker to connect to your IP, check your tracker manual.

## Rabbitmq

Tracker events are published to a single topic exchange with the following pattern <protocol_slug>.<event_type>.<device_imei>. eg: h02.location.imei. With this topic exchange your services can listen to events only they care about, examples:

- all event types of the h02 protocol `h02.*.*`
- location events regardless of the protocol and imei `*.location.*`
- events of a specific tracker, by its imei `*.*.8603412412412`

## Architecture

![diagram](./docs/diagram.png "diagram")

The service is open connection for the following ports/protocols

| PORT  | PROTOCOL | TRACKER PROTOCOL             |
| ----- | -------- | ---------------------------- |
| 3003  | TCP      | [H02](./docs/h02/readme.md)  |

## Environment variables

|           name          |                                    meaning                                   | example                           |
|-------------------------|------------------------------------------------------------------------------|-----------------------------------|
| APP_DEBUG               | debug mode, if true will log to debug info to stdout                         | false                             |
| RMQ_URI                 | rabbitmq uri                                                                 | amqp://guest:guest@localhost:5672 |
| TRACKER_EVENTS_EXCHANGE | name of the rabbitmq exchange to publish events on                           | tracker_events                    |
| TRACER_SERVICE_NAME     | name of the service to jaeger                                                | tracker_receiver                  |
| PORT_H02                | port to listen to TCP requests of H02 trackers                               | tracker_receiver                  |
