
# TODOs

- Start creating the native audio support for standalone version
  - We should have a user interface that is capable of selecting the audio input and output devices.
    - We need to be able to list audio input and output devices using the PAL.
    - We need to be able to select audio input and output devices using the PAL and route them to the audio engine.
    - We need an abstraction for the audio engine which supports connecting to either the VST3/CLAP host or a PAL audio device.
      - We need to develop a generic host audio abstraction which can be used to connect to either the VST3/CLAP host or a PAL audio device.
  - We should have a user interface that is capable of selecting the audio sample rate and buffer size.
    - This interface can assume that the audio engine is connected to a PAL audio device, not a VST3/CLAP host. (The host will set the sample rate, buffer size, and number of input/output devices).
    - The interface will need to be able to communicate with the audio engine to get the current sample rate and buffer size.
    - The interface will need to be able to communicate with the audio engine to set the sample rate and buffer size.
    - The interface will need to communicate with the PAL to get/set the current sample rate and buffer size.
Verification:

- We should write E2E tests using the test-e2e application. Test the following:
  - Generate goldens navigating the Input/Output device selection screen.
  - Generate goldens navigating the Sample Rate/Buffer Size selection screen.
  - Validate input/output selection suceeds and fails gracefully.
  - Validate sample rate/buffer size selection suceeds and fails gracefully.
  - Validate that the audio engine is connected to the audio device and that audio is being processed.