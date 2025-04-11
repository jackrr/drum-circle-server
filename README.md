
# Drum Circle (Server)

## Todo

### V0

- Generate short unique id for users and circles (autoincrement to start?)

## Contracts

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
