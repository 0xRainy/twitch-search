# Search Twitch.tv streams and show a list in the terminal

Usage: 

```
# Category search
Enter a term and choose a result from the returned list of categories


# Stream search
Enter a term to search stream titles in the chosen category
```

*Note:* requires two env vars set to a valid OAuth token and client id:
* `TWITCH_CLIENT_ID`
    To get a client ID register an app (this app) here:
    https://dev.twitch.tv/console/apps/create
    OAuth Redirect URLs can be set to https://localhost
* `TWITCH_TOKEN`
    To get an app OAuth token run:
    * curl -X POST 'https://id.twitch.tv/oauth2/token' \
-H 'Content-Type: application/x-www-form-urlencoded' \
-d 'client_id=<your client id goes here>&client_secret=<your client secret goes here>&grant_type=client_credentials'
