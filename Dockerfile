FROM golang:1.22

WORKDIR /workspace

COPY . .

RUN go build -o /bin/futhorc ./cmd/futhorc