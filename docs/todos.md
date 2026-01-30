
# TODOs

- Test for audio selection dropdown, let's fill with mock data so if host audio inputs change, we can still test.


TODO Homelab setup:

 1. Install GitLab on Unraid:                                                                                     
     * Search for gitlab-ce in the Unraid Community Applications store.                                           
     * Set it to run on a custom port (e.g., 8080) if 80 is used.                                                 
 2. Setup Runners:                                                                                                
     * Linux: Install gitlab-runner docker container on Unraid. Register it with the token from your new GitLab   
       instance. Use the docker executor.                                                                         
     * Windows: Spin up a Windows VM. Download the binary runner. Register it with the tag windows-vm and shell   
       (or powershell) executor.                                                                                  
     * Mac: Download the binary runner on your Mac. Register it with the tag macos-mini and shell executor.       
 3. Push Code:                                                                                                    
     * Add your new self-hosted GitLab remote: git remote add lab http://<unraid-ip>:8080/user/project.git       █
     * Push: git push lab main                                                                                   █
                                                                                                                 █
This setup gives you total control, leverages your powerful Unraid hardware for the heavy lifting                █
(Linux/Windows), and keeps the UI clean and professional.  
