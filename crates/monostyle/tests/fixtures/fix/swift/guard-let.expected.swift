import Foundation

func parse(value: String?) -> String {
    guard let unwrapped = value else {
        return "missing"
    }

    let result = "got \(unwrapped)"
    print(result)

    return result
}

print(parse(value: "data"))
print(parse(value: nil))
