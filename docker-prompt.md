Let's do the following, create a task list to make sure each of the following are complete:

- We should add a buiild flag which optionally enables building and running E2E tests within a docker environment
- When the flag is passed to the build command, the E2E build should also create the docker image(s) it depends on.
- When the flag is passed to the test command, E2E tests should spin up the docker image and run themselves from within the docker image.

To verify the above:
- Please run the build command passing the --docker flag to ensure the build creates the build file.
- Please run the build command without passing the --docker flag to ensure the build works without creating a docker image.
- Pleas run the tests passing the --docker flag to ensure they pass and run within the docker image.
-- Please runn the tests without passing the --docker flag to ensure they run natively and pass.