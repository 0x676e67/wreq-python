import sys
import time

import wreq
import wreq.blocking


def main():
    with wreq.blocking.get("https://httpbin.io/stream/20") as resp:
        with resp.stream() as streamer:
            for chunk in streamer:
                if isinstance(chunk, wreq.HeaderMap):
                    print("Trailers:", chunk)
                else:
                    sys.stdout.buffer.write(chunk)
                time.sleep(0.1)


if __name__ == "__main__":
    main()
