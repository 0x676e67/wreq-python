from wreq import Client
from wreq.header import HeaderMap

if __name__ == "__main__":
    headers = HeaderMap()
    # Add Content-Type header
    headers.insert("Content-Type", "application/json")
    # Add Accept header (first value)
    headers.insert("Accept", "application/json")
    # Add Accept header (second value)
    headers.insert("Accept", "text/html")
    # Get all values for 'Accept' header
    print("All Accept:", list(headers.get_all("Accept")))
    # Get the value for 'Content-Type' header
    print("Content-Type:", headers.get("Content-Type"))
    # Print total number of values in the map
    print("len (all values):", headers.len())
    # Print number of unique keys in the map
    print("keys_len (unique keys):", headers.keys_len())
    # Check if the map is empty
    print("is_empty:", headers.is_empty())
    # Print the entire header map
    print(headers)
    # Clear all headers
    headers.clear()
    print("After clear, is_empty:", headers.is_empty())

    # The headers of a client can be changed after it has been created
    client = Client(headers={"x-api-key": "secret"})
    # Add a header, or replace the value of one that is already there
    client.headers.update({"x-request-id": "1"})
    print("Client headers:", client.headers)
    # Replace the headers of the client altogether
    client.headers = {"x-api-key": "another-secret"}
    print("Client headers:", client.headers)
