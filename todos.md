
# TODOs

- Create infrastructure to test this in Virtualbox on MacOS so it doesn't keep opening popups (we added linux in docker support)
- Potentially rather than having to do sleep waits in tests which may be flaky, we could update the event handler and event loop to support callbacks when events are handled. Then tests would receive an ACK that the event was handled.