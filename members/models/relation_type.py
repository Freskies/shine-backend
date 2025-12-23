from django.db import models


class RelationType(models.TextChoices):
    FATHER = "father", "Father"
    MOTHER = "mother", "Mother"

    SON = "son", "Son"
    DAUGHTER = "daughter", "Daughter"
    CHILD = "child", "Child"

    BROTHER = "brother", "Brother"
    SISTER = "sister", "Sister"
    SIBLING = "sibling", "Sibling"

    HUSBAND = "husband", "Husband"
    WIFE = "wife", "Wife"
    SPOUSE = "spouse", "Spouse"
    PARTNER = "partner", "Partner"
    FIANCE = "fiance", "Fiancé/Fiancée"

    GRANDFATHER = "grandfather", "Grandfather"
    GRANDMOTHER = "grandmother", "Grandmother"
    GRANDPARENT = "grandparent", "Grandparent"
    GRANDSON = "grandson", "Grandson"
    GRANDDAUGHTER = "granddaughter", "Granddaughter"
    GRANDCHILD = "grandchild", "Grandchild"

    AUNT = "aunt", "Aunt"
    UNCLE = "uncle", "Uncle"
    NEPHEW = "nephew", "Nephew"
    NIECE = "niece", "Niece"
    COUSIN = "cousin", "Cousin"

    STEPFATHER = "stepfather", "Stepfather"
    STEPMOTHER = "stepmother", "Stepmother"
    STEPSON = "stepson", "Stepson"
    STEPDAUGHTER = "stepdaughter", "Stepdaughter"
    STEPSIBLING = "stepsibling", "Stepsibling"

    FOSTER_PARENT = "foster_parent", "Foster parent"
    FOSTER_CHILD = "foster_child", "Foster child"

    GUARDIAN = "guardian", "Guardian"
    CAREGIVER = "caregiver", "Caregiver"
    WARD = "ward", "Ward"

    IN_LAW = "in_law", "In-law"
    FATHER_IN_LAW = "father_in_law", "Father-in-law"
    MOTHER_IN_LAW = "mother_in_law", "Mother-in-law"
    SON_IN_LAW = "son_in_law", "Son-in-law"
    DAUGHTER_IN_LAW = "daughter_in_law", "Daughter-in-law"
    BROTHER_IN_LAW = "brother_in_law", "Brother-in-law"
    SISTER_IN_LAW = "sister_in_law", "Sister-in-law"

    FRIEND = "friend", "Friend"
    COLLEAGUE = "colleague", "Colleague"
    EMERGENCY_CONTACT = "emergency_contact", "Emergency contact"

    OTHER = "other", "Other"
