def summarize(data):
    if (size := len(data)) > 3:
        return f"large: {size}"

    while (chunk := data.pop(0)) is not None:
        print(chunk)


    return "done"
