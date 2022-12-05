<div align="center">
  <a href="https://raw.githubusercontent.com/torcoste/relearn-rs">
    <img src="https://raw.githubusercontent.com/torcoste/orderbook-aggregator/main/images/client-screencast.gif" alt="Client screencast" width="600" height="378">
  </a>
  <h3 align="center">orderbook-aggregator</h3>
  <p align="center">
    Aggregator of order books from different cryptocurrency exchanges
  </p>
</div>

<!-- USAGE EXAMPLES -->
## Usage

To start the server, run the command:
   ```sh
   ./run-server.sh
   ```
You can configure the server using environment variables. For example:
   ```sh
   PORT=10001 ./run-server.sh
   ```
The following env variables are supported:
- PORT
- SYMBOL
- DEPTH
- DATA_LIFETIME_MS
- BINANCE_API_BASE_URL
- BITSTAMP_API_URL

To start the client (table view), run the command:
   ```sh
   ./run-client.sh
   ```

It is also possible to specify the port on which the gRPC server is located using env var:

   ```sh
   PORT=10001 ./run-client.sh
   ```

