
# TODOs

Let's work on the audio interface selection interface.

We should use a drop-down menu to select the audio interface.  The drop-down menu should be populated with the audio interfaces that are available on the system.  The drop-down menu should be updated when the audio interfaces are updated.

The item currently hovered over should be highlighted.  The item currently selected should be highlighted differently.

We should verify this works using the test-e2e crate. We should generate goldens for the audio interface selection screen for each of the steps of interacting with the drop-down menu.  The steps are:

1. Open the audio interface selection screen
2. Hover over the first item in the drop-down menu
3. Hover over the second item in the drop-down menu
4. Hover over the third item in the drop-down menu
5. Select the first item in the drop-down menu
6. Select the second item in the drop-down menu
7. Select the third item in the drop-down menu

Rather than continue to expand test-e2e application directory, let's create an example/ directory within the crate and create use the example binary to test the audio interface selection screen.