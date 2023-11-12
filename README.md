
# Bitcoin Price Telegram Bot

Small rust bot that periodically lets users know if the price of bitcoin
has gone up or down by over 3% in the last hour.

This is a small PoC project with no real merit for real world production
purposes. I have spent no more than 10 hours on this over the course of
4 days. Additions and PR's always welcome.

# Requires
- Coin API token
- Telegram bot token and a configured bot
- A redis server because CoinAPI is limited to 100 requests per day
- The chat ID of the group you want to use this with
