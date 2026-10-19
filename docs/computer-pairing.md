# Pair computers

Pairing lets another Miilo device read observations from sessions you have already
selected on a computer. The computer must stay awake, reachable and collecting.
A private network such as Tailscale can provide connectivity; Miilo does not
configure it, expose public ports or install a network service automatically.

1. Select the sessions to observe with `miilo local setup`, then run
   `miilo local run` and keep it running.
2. In another terminal, start sharing with your computer's actual private address:

   ```sh
   miilo local share --bind PRIVATE_IP:38761 --url https://PRIVATE_IP:38761 --name "Workstation"
   ```

3. On that same computer, run `miilo local pair invite`. Transfer the invitation
   privately to the receiving device. It is a secret and expires in five minutes.
4. On a widget device, select **Settings → Pair computer**. Paste the invitation,
   name the receiving device, and compare its displayed code with
   `miilo local pair requests` on the host.
5. Only when the codes match, approve on the host:
   `miilo local pair approve REQUEST_ID COMPARISON_CODE`. The receiver then asks
   which of the shared systems to display. Existing connections are preserved.

A CLI receiver can instead run `miilo local pair connect "Laptop"`, paste the
invitation on stdin, and use `miilo local pair list` and
`miilo local pair snapshot HOST_ID` to inspect its connection.

Use `miilo local pair deny REQUEST_ID` to reject a request. List authorized devices
with `miilo local pair devices`, and revoke one with
`miilo local pair revoke DEVICE_ID`. Credentials expire after 30 days; pair again
to renew. Forgetting a receiver's local connection does not revoke its credential;
revoke it on the host as well. Stop sharing with Ctrl+C.

Only selected observation snapshots are exposed, including their work summaries
and evidence. Choose sessions accordingly. Remote execution, approvals, agent
logins and model credentials are not shared. Clients pin the host certificate;
an unexpected certificate change fails closed. Host state is stored under
`~/.local/state/miilo/pairing`; widget credentials use its private configuration
directory. Files are owner-only, not encrypted. Protect device backups accordingly.

Sharing must be restarted explicitly after a reboot. A lost connection or stale
collector is an error, not proof that the agent is idle. Restore collection and
refresh the receiver. Changing the host origin requires a new host identity and
pairing; preserve the original identity directory when restarting at the same URL.
