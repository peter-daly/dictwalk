class DictWalkError(Exception):
    pass


class DictWalkParseError(DictWalkError):
    def __init__(self, path: str, token: str | None, message: str):
        super().__init__(f"{message} (path='{path}', token='{token}')")
        self.path = path
        self.token = token
        self.message = message


class DictWalkOperatorError(DictWalkError):
    pass


class DictWalkResolutionError(DictWalkError):
    def __init__(self, path: str, token: str | None, message: str):
        super().__init__(f"{message} (path='{path}', token='{token}')")
        self.path = path
        self.token = token
        self.message = message
