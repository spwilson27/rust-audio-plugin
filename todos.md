TODO:

- Fixup the screenshot.png file dump. 
 - We should add support to the debug server to dump the screen via a command.
    - The dump command should receive the screenshot as a return value.
 - Generate goldens should use the dump RPC to get the screenshot.
 - Test cases should use the dump RPC to get their screenshot.

- Update the screen so it doesn't take focus when it is created.