import Foundation

enum Payload {
    case text(String)
    case count(Int)
}

func render(_ payload: Payload) -> String {
    let label: String
    switch payload {
    case .text(let value):
        label = "text {wrapped} \(value)"
    case .count(let number):
        label = "count \(number)"
    }
    print(label)



    return label
}

render(.text("brace {body}"))
