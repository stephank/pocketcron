# pocketcron

A tiny cronjob runner. Great as a 'sidecar' process in your application.

- No syslog. No email. Cronjobs inherit stdout & stderr from pocketcron.

- Cronjobs inherit environment variables from pocketcron. (Does not support
  variables in crontab files.)

- Supports only the user crontab format. (No username field, like in system
  crontab files.)

- Not a deamon. Always runs in the foreground.
