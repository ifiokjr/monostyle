fun prices() {
    val raw = """
cost ${'$'}5
template ${'$'}{kept}
"""
    val label = "total ${'$'}{'$'}x"

    println(raw + label)
}
