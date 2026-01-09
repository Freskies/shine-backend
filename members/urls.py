from django.urls import path
from . import views

urlpatterns = [
    path("", views.get_members),
    path("<int:member_id>", views.get_member),
    path("new", views.create_member),
    path("relationship/new", views.create_relationship),
]
