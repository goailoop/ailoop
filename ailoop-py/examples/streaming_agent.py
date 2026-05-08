"""
Complete runnable example: ailoop-py streaming agent with WebSocket.

Registers handlers before entering async context manager, subscribes to a channel,
sends an ask, and waits for the correlated human response before exiting cleanly.

Run (requires a running ailoop server):
    python examples/streaming_agent.py
"""
import asyncio

from ailoop import AiloopClient


async def main() -> None:
    pending: dict[str, asyncio.Future] = {}
    stop = asyncio.Event()

    async def on_message(data: dict) -> None:
        content = data.get("content", {})
        msg_type = content.get("type")
        cid = data.get("correlation_id")
        if msg_type == "response" and cid and cid in pending:
            pending[cid].set_result(data)
            stop.set()
        else:
            print(f"[message] type={msg_type} channel={data.get('channel')}")

    async def on_connection(event: dict) -> None:
        print(f"[connection] {event['type']}")

    client = AiloopClient("http://127.0.0.1:8080", channel="public")
    # Handlers MUST be registered before entering the async context manager.
    # __aenter__ immediately calls connect_websocket(), which spawns the
    # background receive loop — any handler registered after that point can
    # miss messages (including the initial "connected" event).
    client.add_message_handler(on_message)
    client.add_connection_handler(on_connection)

    async with client:
        await client.subscribe_to_channel("public")
        sent = await client.ask("Proceed with deployment?", timeout=120)
        fut: asyncio.Future = asyncio.get_event_loop().create_future()
        pending[str(sent.id)] = fut
        await stop.wait()
        reply = fut.result()
        print(f"[reply] {reply['content'].get('answer')}")


if __name__ == "__main__":
    asyncio.run(main())
