# Drum Circle (Server)

Rust server supporting the [drum circle client app](https://github.com/jackrr/drum-circle-app).

## Dev setup

1. Clone this repo.

1. Ensure you have rust installed on your system.

1. Start the server (this command will install any dependencies and compile)

```bash
cargo run
```

## Contributing

See the [Issues](https://github.com/jackrr/drum-circle-app/issues) tab
to find proposed features and known issues. You are welcome to suggest
a solution on one of those, put up a PR, or open a new issue to
suggest additional changes!

## API Contracts

- Create Circle (Client -> Server)
```json
{
	"name": "new_circle"
}
```
- Create Circle Response (Server -> Client)
```json
{
	"name": "circle_created",
	"circle_id": "abc123"
}
```
- Join Circle (Client -> Server)
```json
{
	"name": "join_circle",
	"circle_id": "abc123"
}
```
- Membership response (Server -> Client)
```json
{
	"name": "circle_discovery",
	"circle_id": "abc123",
	"members": ["user1", "user2", ...] // IDs of existing members
}
```
- New member RTC offer (Client -> Server -> Client)
```json
{
	"name": "new_member_rtc_offer",
	"circle_id": "abc123",
	"member_id": "user123", // ID of joiner
	"sdp": ...base64-encoded SDP JSON...
}
```
- New member RTC answer (Client -> Server + Forwarded to joiner client)
```json
{
	"name": "new_member_rtc_answer",
	"circle_id": "abc123",
	"member_id": "user123", // ID of joiner
	"sdp": ...base64-encoded SDP JSON...
}
```
- ICE Candidate Broadcast (Client -> Server -> Clients)
```json
{
	"name": "ice_candidate",
	"circle_id": "abc123",
	"member_id": "user123", // ID of ICE owner
	"ice": ...ice JSON...
}
```
