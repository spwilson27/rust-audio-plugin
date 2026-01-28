Let's start working on a UI widget framework. Let's start by developing a plan for how you will implement it.

We should support the following widgets:
- A selectable and editable textbox
- A button
- A slider
- A knob

Each of these should support automated tests to ensure they behave as expected. Thus they should support receiving events through the generic event router. 

The debug backend should be able to send mouse and keyboard input to these widgets to inject behavior without the need for interacting with the GUI directly.
