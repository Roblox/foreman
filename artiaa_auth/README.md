# artiaa_auth
Reads artifactory authentication using the following schema:

```
{
  "$schema": "https://json-schema.org/draft/2019-09/schema",
  "type": "object",
  "title": "ArtiAA Token File Format",
  "description": "This is the format of the ArtiAA Token File, a standard output that contains the Artifactory tokens for the user. ",
  "properties": {
    "tokens": {
      "title": "Tokens URL Map",
      "description": "Map of URLs and the credentials that apply to them.\nThe key of each value is the base domain URL the credential applies to, such as 'artifactory.rbx.com'.\nArtiAA will add an entry for each URL it has been logged into.",
      "type": "object",
      "additionalProperties": {
        "title": "URL Token Element",
        "type": "object",
        "properties": {
          "username": {
            "title": "Username",
            "description": "The username to use for accessing the server, if a username is necessary.\nMay be an empty string.",
            "type": "string"
          },
          "token": {
            "title": "Token",
            "description": "The token to use for accessing the server.",
            "type": "string"
          }
        },
        "additionalProperties": false
      }
    }
  }
}
```