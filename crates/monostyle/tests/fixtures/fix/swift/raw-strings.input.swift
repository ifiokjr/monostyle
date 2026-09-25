import Foundation

func rawDemo() -> String {
    let interpolated = "value"
    let first = #"raw \#(interpolated) string"#
    let second = ##"no \#(escape) here \n still raw"##
    print(first)
    print(second)



    return first + second
}

let output = rawDemo()
print(output)
