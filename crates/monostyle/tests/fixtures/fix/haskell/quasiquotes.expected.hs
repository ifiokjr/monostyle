module QQ where

import Language.Haskell.TH.Quote (QuasiQuoter)

hash :: QuasiQuoter
hash = QuasiQuoter quoteExp quotePat undefined undefined

value :: String
value = [hash| abc {kept} |]
