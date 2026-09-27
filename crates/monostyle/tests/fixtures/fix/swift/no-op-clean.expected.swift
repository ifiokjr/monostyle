import Foundation

func tricky() -> String {
    let name = "world"
    let block = """
    value \(name) with {braces} and "quotes"
    second \(name) line
    """
    let raw = #"literal \#(name) here"#
    let joined = block + raw

    return joined
}

let result = tricky()
print(result)
