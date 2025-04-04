
# Drum Circle (Server)

## Todo

### V0

- Generate short unique id for users and circles (autoincrement to start?)
- 

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
- Join offers (Client -> Server)
```json
{
	"name": "circle_join_offers",
	"circle_id": "abc123",
	"sdps": [{user_id: "user1", sdp: ...base64-encoded SDP JSON...}, 
	         {user_id: "user2": sdp: ...},
			 ...]
}
```
- New member RTC offer (Server -> Client)
```json
{
	"name": "new_member_rtc_offer",
	"member_id": "user123", // ID of joiner
	"sdp": ...base64-encoded SDP JSON...
}
```
- New member RTC answer (Client -> Server)
```json
{
	"name": "new_member_rtc_answer",
	"member_id": "user123", // ID of joiner
	"sdp": ...base64-encoded SDP JSON...
}
```
