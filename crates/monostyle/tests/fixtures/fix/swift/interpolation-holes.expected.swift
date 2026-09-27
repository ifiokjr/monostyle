import Foundation

func describeItems() -> String {
    func find(_ key: String) -> String {
        return "value-\(key)"
    }

    let items = "items: \(find("key")) done"
    print(items)

    return items
}

describeItems()
