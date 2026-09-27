import Foundation

func greeting(name: String) -> String {
    let template = """
    Hello \(name),
    this is a {braced} note with "quotes" inside.
    Second line with \(name) again.
    """

    print(template)

    return template
}

let message = greeting(name: "World")
print(message)
