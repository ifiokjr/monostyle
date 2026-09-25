import asyncio


async def collect(streams):
    results = []

    async with asyncio.Lock():
        for stream in streams:
            async for chunk in stream:
                results.append(chunk)


    return results
