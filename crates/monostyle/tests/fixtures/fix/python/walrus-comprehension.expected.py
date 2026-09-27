def big_names(people):
    large = [name for person in people if (length := len(person)) > 3 for name in (person,)]


    return large
