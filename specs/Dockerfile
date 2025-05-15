FROM node:18-alpine

RUN npm install -g @stoplight/prism-cli

WORKDIR /app

COPY postman/openapi.yaml /app/openapi.yaml

EXPOSE 4010

CMD ["prism", "mock", "-h", "0.0.0.0", "openapi.yaml"]
