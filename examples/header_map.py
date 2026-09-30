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
    print("All Accept:", [str(value, "ascii") for value in headers.get_all("Accept")])
    # Get the value for 'Content-Type' header
    content_type = headers.get("Content-Type")
    if content_type is not None:
        print("Content-Type:", str(content_type, "ascii"))
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
